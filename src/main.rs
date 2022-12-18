#![allow(non_snake_case)]

use std::fs::OpenOptions;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread::{sleep, JoinHandle};
use std::time::Duration;

use std::fs::File;
use std::io;
use std::io::Write;
use std::{thread, u128};

use num_bigint::BigUint;
use queues::{IsQueue, Queue};
use rand::Rng;

mod message;
mod r#static;
mod display;
mod resolv;

use r#static::*;
use message::*;
use resolv::*;


/// Counts how many posible distinct numbers can this program (using current filters) generate
pub fn count_posibilites() -> u128 {
    let mut count: u128 = 0;
    for (s, e) in NO_GO_RANGES {
        count += e - s;
    };
    count -= NEXT_PRIME;

    println!("{}", count);

    return count;
}
    
fn write_worker(mut out_file: File, out_queue: Arc<Mutex<Queue<MessageToWrite>>>) {
    loop {
        if let Ok( message ) = out_queue.lock().unwrap().remove() {
            match message {
                MessageToWrite::ToWrite(ip, host) => {
                    writeln!(&mut out_file, "{a}, {b}", a=ip, b=host).expect("Can't write to out file!");
                },
                MessageToWrite::End => { break },
                MessageToWrite::EmptyQueue => todo!(),
            };
        } else {
            if WRITER____STOP_SIGNAL.load(Ordering::Relaxed) { break };
            sleep(Duration::from_millis(SLEEP_TIME * 10));
        };
    };
}

fn generate(to_check: Arc<Mutex<Queue<MessageToCheck>>>, mut skip: BigUint, mut num: BigUint, last: BigUint, zip: BigUint, mut zip_flag: bool) -> (BigUint, u128) {
    let mut c: u128 = 0;
    
    let first_number: BigUint = num.clone();

    let mut send: bool;

    loop { // Generates IIPs for the query worker threads
        if GENERATOR_STOP_SIGNAL.load(Ordering::Relaxed) { break };

        let can_go: bool = to_check.lock().unwrap().size() < QUEUE_LIMIT * 10;

        if can_go {
            c += 1;
            
            if skip == BigUint::from(0u128) {
                send = true;
            } else {
                skip -= BigUint::from(1u128);
                send = false;
            };

            if zip_flag {
                if num != zip {
                    send = false;
                } else {
                    zip_flag = false;
                }
            } 

            if send { to_check.lock().unwrap().add( MessageToCheck::ToCheck(c, num.clone()) ).unwrap(); };

            num = (BigUint::from(A_PRIMA) * num + BigUint::from(C_PRIMA)) % BigUint::from(M_PRIMA);
            
            if num == first_number {
                println!("We went all the way arround!!!1!!11!1one!!1!111");
                break;
            };
            
            if num == last {
                println!("We reached the stipulated end!");
                break;
            }
            
            // #[cfg(debug_assertions)] println!("{}", format!("to_check queue size is currently: {} items long.", to_check.lock().unwrap().size()));
        } else {
            sleep(Duration::from_secs(SLEEP_TIME / 2));
        };
    };

    to_check.lock().unwrap().add( MessageToCheck::End ).unwrap();

    return (num, c);
}

fn display_status(to_write: Arc<Mutex<Queue<MessageToWrite>>>, to_check: Arc<Mutex<Queue<MessageToCheck>>>) {
    
    let mut stop_signal_status: [bool;  4];
    let mut queue_sizes:        [usize; 2];
    let mut last_items:         (MessageToCheck, MessageToWrite);


    loop {
        stop_signal_status = [
            GENERATOR_STOP_SIGNAL.load(Ordering::Relaxed),
            QUERYER___STOP_SIGNAL.load(Ordering::Relaxed),
            WRITER____STOP_SIGNAL.load(Ordering::Relaxed),
            DISPLAY___STOP_SIGNAL.load(Ordering::Relaxed),
        ];

        {
            let inquee   = to_check.lock().unwrap(); 
            let outqueue = to_write.lock().unwrap();

            queue_sizes = [
                inquee.size(),
                outqueue.size(),
            ];

            last_items = (
                match inquee.peek() {
                    Ok(message) => { message },
                    Err(_) => { MessageToCheck::EmptyQueue },
                },
                match outqueue.peek() {
                    Ok(message) => { message },
                    Err(_) => { MessageToWrite::EmptyQueue },
                },
            );
        };

        println!("{}", format!("signal status: {:?}; queue sizes: {:?}; last times: {:?}", stop_signal_status, queue_sizes, last_items));
        io::stdout().flush().expect("\n\rUnable to flush stdout!");

        if DISPLAY___STOP_SIGNAL.load(Ordering::Relaxed) { break };

        sleep(Duration::from_secs_f32(0.3));
    };
}


fn launch_display_threads(d_to_write: Arc<Mutex<Queue<MessageToWrite>>>, d_to_check: Arc<Mutex<Queue<MessageToCheck>>>) -> Option<JoinHandle<()>> {    
    let display_thread: Option<JoinHandle<()>>;

    if cfg!(debug_assertions) {
        display_thread = std::option::Option::Some(thread::Builder::new().name("DisplayThread".into()).spawn(move || {
            display_status(d_to_write, d_to_check);
        }).unwrap());
    } else {
        display_thread = std::option::Option::None;
    };

    thread::Builder::new().name("DisplayUpdateThread".into()).spawn(move || loop {
        if DISPLAY___STOP_SIGNAL.load(Ordering::Relaxed) { break };
        sleep(Duration::from_secs_f32(0.1));
        io::stdout().flush().expect("\n\rUnable to flush stdout!");
    }).unwrap();

    return display_thread;
}

fn launch_generator_thread(to_check: Arc<Mutex<Queue<MessageToCheck>>>, skip: BigUint, num: BigUint, last: BigUint, zip: BigUint, zip_flag: bool) -> JoinHandle<(BigUint, u128)> {
    return thread::Builder::new().name("GeneratorThread".into()).spawn(move || { return generate(to_check, skip, num, last, zip, zip_flag); }).unwrap();
}

fn launch_write_thread(queuee: Arc<Mutex<Queue<MessageToWrite>>>, out_file: File) -> JoinHandle<()> {
    return thread::Builder::new().name("WriterThread".into()).spawn(move || { write_worker(out_file, queuee); }).unwrap();
}

fn launch_worker_threads(worker_threads: &mut Vec<JoinHandle<()>>, tc: Arc<Mutex<Queue<MessageToCheck>>>, tw: Arc<Mutex<Queue<MessageToWrite>>>) {
    for n in 0..(CORES * 4) { // Starts query worker threads
        let (tc_, tw_) = (tc.clone(), tw.clone());
        worker_threads.push(thread::Builder::new().name(format!("QueryerThread#{}", n)).spawn(move || { resolv_worker(tc_, tw_); }).unwrap());
    };
}

/// This function will panic if you hit Ctrl + C more than 255 times xd
fn ctrl_c_handler() {
    println!("{}", match CTL_C_C___STOP_SIGNAL.load(Ordering::Relaxed) {
        0 => { GENERATOR_STOP_SIGNAL.store(true, Ordering::Relaxed); "Keyboard Interrupt recieved, signaling generator thread to stop." },
        1 => { QUERYER___STOP_SIGNAL.store(true, Ordering::Relaxed); "Keyboard Interrupt recieved, signaling query threads to stop." },
        2 => { WRITER____STOP_SIGNAL.store(true, Ordering::Relaxed); "Keyboard Interrupt recieved, signaling writer thread to stop!" },
        3 => { DISPLAY___STOP_SIGNAL.store(true, Ordering::Relaxed); "Keyboard Interrupt recieved, signaling display thread to stop!" },
        _ => { "Keyboard Interrupt recieved, signaling no one, lol." },
    });

    CTL_C_C___STOP_SIGNAL.fetch_add(1u8, Ordering::Relaxed);
}

fn set_cc_handler() {
    ctrlc::set_handler( move || { ctrl_c_handler(); }).expect("Error setting Ctrl-C handler");
}

#[allow(unused_variables)]
fn main() {
    let mut num:       BigUint                           = BigUint::from(rand::thread_rng().gen::<u128>());

    let mut skip:      BigUint                           = BigUint::from(0u128);
    
    let mut last:      BigUint                           = BigUint::from(LAST_NUMBR);
    
    let mut zip:       BigUint                           = BigUint::from(0u128);
    let mut zip_flag:  bool                              = false;

    let     c_last:    BigUint;
    let     num_last:  u128;

    let     out_file:  File;
    
    let     b:         &std::path::Path                  = std::path::Path::new(OUT_FILE_NAME);
    
    let     to_write:  Arc<Mutex<Queue<MessageToWrite>>> = Arc::new(Mutex::new(Queue::new()));
    let     to_check:  Arc<Mutex<Queue<MessageToCheck>>> = Arc::new(Mutex::new(Queue::new()));

    //parse cli args
    if let Some(r_seed) = std::env::args().nth(1) {
        num = r_seed.parse().expect("Invalid Seed (seed must be an unsinged int)")
    };

    if let Some(r_skip) = std::env::args().nth(2) { skip = r_skip.parse().expect("Invalid skip number (skip number must be an unsinged int)"); };
    if let Some(r_last) = std::env::args().nth(3) { last = r_last.parse().expect("Invalid last number (last number must be an unsinged int)"); };
    if let Some(r_zip)  = std::env::args().nth(4) {
        zip = r_zip.parse::<BigUint>().expect("fuck");
        zip_flag = true;
    };

    assert!(last > skip, "Last number must be greater than the number of skipped iterations.");

    set_cc_handler();
    
    let mut worker_threads:    Vec<thread::JoinHandle<()>> = Vec::new();
    
    let generator_thread:  JoinHandle<(BigUint, u128)>;
    let display_thread:    std::option::Option<JoinHandle<()>>;
    let write_thread:      JoinHandle<()>;

    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move | panic_info | {
        orig_hook(panic_info);
        std::process::exit(1);
    }));

    if b.exists() {
        out_file = OpenOptions::new().append(true).open(b.clone()).expect("Could not open existing file!");
        println!("{}", format!("Using existing file: {}", b.display()));
    } else {
        out_file = OpenOptions::new().create(true).write(true).open(b.clone()).expect("Could not create output file!");
        println!("{}", format!("Created new file: {}", b.display()));
        to_write.lock().unwrap().add(MessageToWrite::ToWrite(String::from("IP"), String::from("HOSTNAME"))).unwrap();
    };

    println!("{}", format!("The seed is {}", num));
        
    num = (BigUint::from(A_PRIMA) * num + BigUint::from(C_PRIMA)) % BigUint::from(M_PRIMA);
    println!("{}", format!("first number is: {}", num.clone()));
    
    println!("Starting threads!");

    launch_worker_threads(&mut worker_threads, to_check.clone(), to_write.clone());

    write_thread     = launch_write_thread(to_write.clone(), out_file);
    display_thread   = launch_display_threads(to_write.clone(), to_check.clone());
    generator_thread = launch_generator_thread(to_check.clone(), skip.clone(), num.clone(), last, zip, zip_flag);

    (c_last, num_last) = generator_thread.join().unwrap();
    
    #[cfg(debug_assertions)] println!("We got hereeeeeeeeee");
    
    while let Some(cur_thread) = worker_threads.pop() {
        #[cfg(debug_assertions)] println!("{}", format!("waiting for worker thread: {:?}.", cur_thread.thread().id()));
        cur_thread.join().unwrap();
    };
    
    to_write.lock().unwrap().add( MessageToWrite::End ).unwrap();
    
    #[cfg(debug_assertions)] println!("waiting for writer thread.");
    
    write_thread.join().unwrap();
    
    DISPLAY___STOP_SIGNAL.store(true, Ordering::Relaxed);
    
    if let Some(display_thread) = display_thread {
        display_thread.join().unwrap();
    };
    
    println!("{}", format!("We started @ {} Iterations",       skip));
    println!("{}", format!("The last number was => {}",        c_last));
    println!("{}", format!("It appeared after {} iterations.", num_last));

    println!("End.");

}
