extern crate hwloc;
extern crate libc;
extern crate regex;

use faster_kvs::FasterKv;
use hwloc::{CpuSet, ObjectType, Topology, CPUBIND_THREAD};
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::prelude::FileExt;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::time::{Duration, Instant};

const K_CHECKPOINT_SECONDS: u64 = 30;
const K_COMPLETE_PENDING_INTERVAL: usize = 1600;
const K_REFRESH_INTERVAL: usize = 64;
const K_RUN_TIME: u64 = 360;
const K_CHUNK_SIZE: usize = 3200;
const K_FILE_CHUNK_SIZE: usize = 131072;
const K_INIT_COUNT: usize = 250000000;
const K_TXN_COUNT: usize = 1000000000;

const K_NANOS_PER_SECOND: usize = 1000000000;

const K_THREAD_STACK_SIZE: usize = 4 * 1024 * 1024;

pub enum Operation {
    Read,
    Upsert,
    Rmw,
}

fn cpuset_for_core(topology: &Topology, idx: usize) -> CpuSet {
    let cores = (*topology).objects_with_type(&ObjectType::Core).unwrap();
    match cores.get(idx) {
        Some(val) => val.cpuset().unwrap(),
        None => panic!("No Core found with id {}", idx),
    }
}

pub fn process_ycsb(input_file: &str, output_file: &str) {
    let input = File::open(input_file).expect("Unable to open input file for reading");
    let mut output = File::create(output_file).expect("Unable to create output file");

    let re = Regex::new(r".*usertable user(\d+).*").unwrap();

    let reader = BufReader::new(input);
    for line in reader.lines().map(|l| l.unwrap()) {
        for cap in re.captures_iter(&line) {
            let num: u64 = cap[1].parse().expect("Unable to parse uid");
            output.write(&num.to_be_bytes()).unwrap();
        }
    }
}

pub fn read_upsert5050(key: usize) -> Operation {
    match key % 2 {
        0 => Operation::Read,
        1 => Operation::Upsert,
        _ => panic!(),
    }
}

pub fn rmw_100(_key: usize) -> Operation {
    Operation::Rmw
}

pub fn upsert_100(_key: usize) -> Operation {
    Operation::Upsert
}

pub fn load_files(load_file: &str, run_file: &str) -> (Vec<u64>, Vec<u64>) {
    let load_file = File::open(load_file).expect("Unable to open load file");
    let run_file = File::open(run_file).expect("Unable to open run file");

    let mut buffer = [0; K_FILE_CHUNK_SIZE];
    let mut count = 0;
    let mut offset = 0;

    let mut init_keys = Vec::with_capacity(K_INIT_COUNT);

    println!("Loading keys into memory");
    loop {
        let bytes_read = load_file.read_at(&mut buffer, offset).unwrap();
        for i in 0..(bytes_read / 8) {
            let mut num = [0; 8];
            num.copy_from_slice(&buffer[i..i + 8]);
            init_keys.insert(count, u64::from_be_bytes(num));
            count += 1;
        }
        if bytes_read == K_FILE_CHUNK_SIZE {
            offset += K_FILE_CHUNK_SIZE as u64;
        } else {
            break;
        }
    }
    if K_INIT_COUNT != count {
        panic!("Init file load fail!");
    }
    println!("Loaded {} keys", count);

    let mut count = 0;
    let mut offset = 0;

    let mut run_keys = Vec::with_capacity(K_TXN_COUNT);

    println!("Loading txns into memory");
    loop {
        let bytes_read = run_file.read_at(&mut buffer, offset).unwrap();
        for i in 0..(bytes_read / 8) {
            let mut num = [0; 8];
            num.copy_from_slice(&buffer[i..i + 8]);
            run_keys.insert(count, u64::from_be_bytes(num));
            count += 1;
        }
        if bytes_read == K_FILE_CHUNK_SIZE {
            offset += K_FILE_CHUNK_SIZE as u64;
        } else {
            break;
        }
    }
    if K_TXN_COUNT != count {
        panic!("Txn file load fail!");
    }
    println!("Loaded {} txns", count);

    (init_keys, run_keys)
}

pub fn populate_store(store: &Arc<FasterKv>, keys: &Arc<Vec<u64>>, num_threads: u8) {
    let topo = Arc::new(Mutex::new(Topology::new()));
    let idx = Arc::new(AtomicUsize::new(0));
    let mut threads = vec![];

    for thread_idx in 0..num_threads {
        let store = Arc::clone(store);
        let idx = Arc::clone(&idx);
        let keys = Arc::clone(&keys);
        let child_topo = topo.clone();

        threads.push(std::thread::spawn(move || {
            {
                // Bind thread to core
                let tid = unsafe { libc::pthread_self() };
                let mut locked_topo = child_topo.lock().unwrap();
                let bind_to = cpuset_for_core(&*locked_topo, thread_idx as usize);
                locked_topo
                    .set_cpubind_for_thread(tid, bind_to, CPUBIND_THREAD)
                    .unwrap();
            }

            let _session = store.start_session();
            let mut chunk_idx = idx.fetch_add(K_CHUNK_SIZE, Ordering::SeqCst);
            while chunk_idx < K_INIT_COUNT {
                for i in chunk_idx..(chunk_idx + K_CHUNK_SIZE) {
                    if i % K_REFRESH_INTERVAL == 0 {
                        store.refresh();
                        if i % K_COMPLETE_PENDING_INTERVAL == 0 {
                            store.complete_pending(false);
                        }
                    }
                    store.upsert(*keys.get(i as usize).unwrap(), &42, i as u64);
                }
                chunk_idx = idx.fetch_add(K_CHUNK_SIZE, Ordering::SeqCst);
            }
            store.complete_pending(true);
            store.stop_session();
        }));
    }
    for t in threads {
        t.join().expect("Something went wrong in a thread");
    }
}

pub fn run_benchmark<F: Fn(usize) -> Operation + Send + Copy + 'static>(
    store: &Arc<FasterKv>,
    keys: &Arc<Vec<u64>>,
    num_threads: u8,
    op_allocator: F,
) {
    let topo = Arc::new(Mutex::new(Topology::new()));
    let idx = Arc::new(AtomicUsize::new(0));
    let done = Arc::new(AtomicBool::new(false));
    let barrier = Arc::new(Barrier::new((num_threads + 1) as usize));
    let mut threads = vec![];

    for thread_id in 0..num_threads {
        let store = Arc::clone(&store);
        let keys = Arc::clone(&keys);
        let idx = Arc::clone(&idx);
        let done = Arc::clone(&done);
        let barrier = Arc::clone(&barrier);
        let topo = Arc::clone(&topo);

        threads.push(
            std::thread::Builder::new()
                .stack_size(K_THREAD_STACK_SIZE)
                .spawn(move || {
                    {
                        // Bind thread to core
                        let tid = unsafe { libc::pthread_self() };
                        let mut locked_topo = topo.lock().unwrap();
                        let bind_to = cpuset_for_core(&*locked_topo, thread_id as usize);
                        locked_topo
                            .set_cpubind_for_thread(tid, bind_to, CPUBIND_THREAD)
                            .unwrap();
                    }

                    let mut reads = 0;
                    let mut upserts = 0;
                    let mut rmws = 0;

                    let _session = store.start_session();

                    barrier.wait();
                    let start = Instant::now();
                    while !done.load(Ordering::SeqCst) {
                        let mut chunk_idx = idx.fetch_add(K_CHUNK_SIZE, Ordering::SeqCst);
                        while chunk_idx >= K_TXN_COUNT {
                            if chunk_idx == K_TXN_COUNT {
                                idx.store(0, Ordering::SeqCst);
                            }
                            chunk_idx = idx.fetch_add(K_CHUNK_SIZE, Ordering::SeqCst);
                        }
                        for i in chunk_idx..(chunk_idx + K_CHUNK_SIZE) {
                            if i % K_REFRESH_INTERVAL == 0 {
                                store.refresh();
                                if i % K_COMPLETE_PENDING_INTERVAL == 0 {
                                    store.complete_pending(false);
                                }
                            }
                            match op_allocator(i) {
                                Operation::Read => {
                                    store.read::<i32>(*keys.get(i).unwrap(), 1);
                                    reads += 1;
                                }
                                Operation::Upsert => {
                                    store.upsert(*keys.get(i).unwrap(), &42, 1);
                                    upserts += 1;
                                }
                                Operation::Rmw => {
                                    store.rmw(*keys.get(i).unwrap(), &5, 1);
                                    rmws += 1;
                                }
                            }
                        }
                    }

                    store.complete_pending(true);
                    store.stop_session();
                    let duration = Instant::now().duration_since(start);

                    println!(
                        "Thread {} completed {} reads, {} upserts and {} rmws in {}ms",
                        thread_id,
                        reads,
                        upserts,
                        rmws,
                        duration.as_millis()
                    );

                    (reads, upserts, rmws, duration.as_nanos())
                })
                .unwrap(),
        )
    }

    barrier.wait();
    let start = Instant::now();
    let mut last_checkpoint = Instant::now();
    let mut num_checkpoints = 0;

    while Instant::now().duration_since(start).as_secs() < K_RUN_TIME {
        std::thread::sleep(Duration::from_secs(1));
        if Instant::now().duration_since(last_checkpoint).as_secs() > K_CHECKPOINT_SECONDS {
            println!("Checkpointing...");
            store.checkpoint();
            num_checkpoints += 1;
            last_checkpoint = Instant::now();
        }
    }

    done.store(true, Ordering::SeqCst);

    let mut total_counts = (0, 0, 0, 0);
    for t in threads {
        let (reads, upserts, rmws, duration) = t.join().expect("Something went wrong in a thread");
        total_counts.0 += reads;
        total_counts.1 += upserts;
        total_counts.2 += rmws;
        total_counts.3 += duration;
    }

    println!(
        "Finished benchmark: {} checkpoints, {} reads, {} writes, {} rmws. {} ops/second/thread",
        num_checkpoints,
        total_counts.0,
        total_counts.1,
        total_counts.2,
        (total_counts.0 + total_counts.1 + total_counts.2)
            / (total_counts.3 as usize / K_NANOS_PER_SECOND)
    )
}
