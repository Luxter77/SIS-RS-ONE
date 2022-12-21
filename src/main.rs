#![allow(non_snake_case)]

use std::fs::OpenOptions;
use std::sync::atomic::Ordering;
use std::thread::{sleep, JoinHandle};
use std::time::Duration;

use std::fs::File;
use std::io::Write;
use std::{thread, u128};

use rand::Rng;

mod generator;
mod r#static;
mod display;
mod message;
mod resolv;

use r#static::*;
use message::{MessageToPrintOrigin, MessageToWrite, MessageToPrint};
use display::{launch_display_thread, launch_status_thread, display};
use generator::{generate, count_posibilites};
use resolv::resolv_worker;


fn write_worker(mut out_file: File) {
    loop {
        if let Ok( message ) = QUEUE_TO_WRITE.get() {
            match message {
                MessageToWrite::ToWrite(ip, host) => { writeln!(&mut out_file, "{a}, {b}", a=ip, b=host).expect("Can't write to out file!"); },
                MessageToWrite::End => { break },
                MessageToWrite::EmptyQueue => todo!(),
            };
        } else {
            if WRITER____STOP_SIGNAL.load(Ordering::Relaxed) { break };
            sleep(Duration::from_millis(SLEEP_TIME * 10));
        };
    };
    display(MessageToPrintOrigin::WriterThread, "[ Write End ]");
}

fn launch_generator_thread(skip: u128, num: u128, last: u128, zip: u128, zip_flag: bool) -> JoinHandle<(u128, u128)> {
    display(MessageToPrintOrigin::MainThread, "[ Launching GeneratorThread ]");
    return thread::Builder::new().name("GeneratorThread".into()).spawn(move || { return generate(skip, num, last, zip, zip_flag); }).unwrap();
}

fn launch_write_thread(out_file: File) -> JoinHandle<()> {
    display(MessageToPrintOrigin::MainThread, "[ Launching WriterThread ]");
    return thread::Builder::new().name("WriterThread".into()).spawn(move || { write_worker(out_file); }).unwrap();
}

fn launch_worker_threads(worker_threads: &mut Vec<JoinHandle<()>>) {
    for n in 0..(CORES * 4) { // Starts query worker threads
        let nam = format!("QueryerThread#{}", n);
        if cfg!(debug_assertions) { display(MessageToPrintOrigin::MainThread, &format!("[ Launching: {} ]", nam.clone())); };
        worker_threads.push(thread::Builder::new().name(nam).spawn(move || { resolv_worker(); }).unwrap());
    };
}

/// This function will panic if you hit Ctrl + C more than 255 times xd
fn ctrl_c_handler() {
    println!("{}", match CTL_C_C___STOP_SIGNAL.load(Ordering::Relaxed) {
        0 => { GENERATOR_STOP_SIGNAL.store(true, Ordering::Relaxed); "[ @CT_C_SIG_HANDLER ][ Keyboard Interrupt recieved, signaling generator thread to stop. ]" },
        1 => { QUERYER___STOP_SIGNAL.store(true, Ordering::Relaxed); "[ @CT_C_SIG_HANDLER ][ Keyboard Interrupt recieved, signaling query threads to stop. ]" },
        2 => { WRITER____STOP_SIGNAL.store(true, Ordering::Relaxed); "[ @CT_C_SIG_HANDLER ][ Keyboard Interrupt recieved, signaling writer thread to stop! ]" },
        3 => { DISPLAY___STOP_SIGNAL.store(true, Ordering::Relaxed); "[ @CT_C_SIG_HANDLER ][ Keyboard Interrupt recieved, signaling display thread to stop! ]" },
        _ => { "[ @CT_C_SIG_HANDLER ][ Keyboard Interrupt recieved, signaling no one, lol. ]" },
    });

    CTL_C_C___STOP_SIGNAL.fetch_add(1u8, Ordering::Relaxed);
}

fn set_cc_handler() {
    ctrlc::set_handler( move || { ctrl_c_handler(); }).expect("Error setting Ctrl-C handler");
}

#[allow(unused_variables)]
fn main() {
    let mut num:       u128                           = rand::thread_rng().gen::<u128>();

    let mut skip:      u128                           = 0u128;
    
    let mut last:      u128                           = LAST_NUMBR;
    
    let mut zip:       u128                           = 0u128;
    let mut zip_flag:  bool                              = false;

    let     c_last:    u128;
    let     num_last:  u128;
    let     out_file:  File;
    
    let     b:         &std::path::Path                  = std::path::Path::new(OUT_FILE_NAME);
    
    //parse cli args
    if let Some(r_seed) = std::env::args().nth(1) {
        num = r_seed.parse().expect("Invalid Seed (seed must be an unsinged int)")
    };
    
    if let Some(r_skip) = std::env::args().nth(2) { skip = r_skip.parse().expect("Invalid skip number (skip number must be an unsinged int)"); };
    if let Some(r_last) = std::env::args().nth(3) { last = r_last.parse().expect("Invalid last number (last number must be an unsinged int)"); };
    if let Some(r_zip)  = std::env::args().nth(4) { zip = r_zip.parse::<u128>().expect("fuck"); zip_flag = true; };
     
    assert!(last > skip, "Last number must be greater than the number of skipped iterations.");
    
    set_cc_handler();
    
    let mut worker_threads:    Vec<thread::JoinHandle<()>> = Vec::new();
    
    let generator_thread:  JoinHandle<(u128, u128)>;
    let display_thread:    JoinHandle<()>;
    
    #[allow(unused_variables)]
    let mut status_thread:     std::option::Option<JoinHandle<()>> = std::option::Option::None;
    
    let write_thread:      JoinHandle<()>;

    let     numspace:  u128                           = count_posibilites(last.clone() - skip.clone());
    
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move | panic_info | {
        orig_hook(panic_info);
        std::process::exit(1);
    }));
    
    if b.exists() {
        out_file = OpenOptions::new().append(true).open(b.clone()).expect("Could not open existing file!");
        println!("{}", format!("[ @MAIN_THREAD      ][ Using existing file: {} ]", b.display()));
    } else {
        out_file = OpenOptions::new().create(true).write(true).open(b.clone()).expect("Could not create output file!");
        println!("{}", format!("[ @MAIN_THREAD      ][ Created new file: {} ]", b.display()));
        QUEUE_TO_WRITE.add(MessageToWrite::ToWrite(String::from("IP"), String::from("HOSTNAME")));
    };

    println!("{}", format!("[ @MAIN_THREAD      ][ The seed is {} ]", num));
    
    // ```(A_PRIMA * num + C_PRIMA) % M_PRIMA``` but ```A_PRIMA * num``` may not fit on a u128
    // so we aply some funky math to keep the numbers down...
    num = (((A_PRIMA % M_PRIMA) * (num % M_PRIMA)) % M_PRIMA) + (C_PRIMA % M_PRIMA);
    
    println!("{}", format!("[ @MAIN_THREAD      ][ First number is: {} ]", num.clone()));
    
    println!("{}", format!("[ @MAIN_THREAD      ][ This run will generate {} valid IPs ]", numspace.clone()));

    println!("{}", "[ @MAIN_THREAD      ][ Starting threads! ]");
    
    println!("{}", "[ @MAIN_THREAD      ][ Launching DisplayThread ]");
    display_thread   = launch_display_thread();
        
    generator_thread = launch_generator_thread(skip.clone(), num.clone(), last, zip, zip_flag);
    write_thread     = launch_write_thread(out_file);
    
    if cfg!(debug_assertions) { status_thread  = launch_status_thread(); };
    
    display(MessageToPrintOrigin::MainThread, "[ Launching WorkerThreads ]");
    launch_worker_threads(&mut worker_threads);
    
    (c_last, num_last) = generator_thread.join().unwrap();
    
    if cfg!(debug_assertions) { display(MessageToPrintOrigin::MainThread, "[ We got hereeeeeeeeee ]"); };
    
    display(MessageToPrintOrigin::MainThread, "[ waiting for worker threads ]");
    while let Some(cur_thread) = worker_threads.pop() {
        if cfg!(debug_assertions) { display(MessageToPrintOrigin::MainThread, &format!("[ waiting for worker thread: {:?} ]", cur_thread.thread().id())); };
        cur_thread.join().unwrap();
    };
    
    QUEUE_TO_WRITE.add( MessageToWrite::End );
    
    display(MessageToPrintOrigin::MainThread, "[ Waiting for writer thread. ]");
    
    write_thread.join().unwrap();
    
    display(MessageToPrintOrigin::MainThread, "[ - - - - - - - - - - - - - ]");
    display(MessageToPrintOrigin::MainThread, &format!("[ We started @ {} Iterations ]",            skip));
    display(MessageToPrintOrigin::MainThread, &format!("[ The last number was => {} ]",             c_last));
    display(MessageToPrintOrigin::MainThread, &format!("[ We found {} records ({} idstinct IPs out of {} in usable space) ]", F_COUNT.get(), F_D_COUNT.get(), numspace));
    display(MessageToPrintOrigin::MainThread, &format!("[ It appeared after {} iterations. ]",      num_last));
    display(MessageToPrintOrigin::MainThread, "[ - - - - - - - - - - - - - ]");

    if let Some(status_thread) = status_thread {
        display(MessageToPrintOrigin::MainThread, "[ Waiting for status thread. ]");
        status_thread.join().unwrap();
    };
    
    display(MessageToPrintOrigin::MainThread, "[ Waiting for display thread. ]");
    QUEUE_TO_PRINT.add(MessageToPrint::End);
    display_thread.join().unwrap();

    println!("[ End. ]");

}
