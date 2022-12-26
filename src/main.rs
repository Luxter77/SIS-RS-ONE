#![allow(non_snake_case)]

use std::fs::OpenOptions;
use std::sync::atomic::Ordering;
use std::thread::JoinHandle;

use std::fs::File;
use std::{thread, u128};

use rand::Rng;

mod generators;
mod generator;
mod r#static;
mod display;
mod workers;
mod message;
mod resolv;

use message::{MessageToPrintOrigin, MessageToWrite, MessageToPrint};
use display::{launch_display_thread, launch_status_thread, display};
use resolv::count_posibilites;
use r#static::*;
use workers::*;

use crate::generators::IPGenerator;

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
    let mut zip:       u32                            = 0u32;
    let mut zip_flag:  bool                           = false;
    
    
    let     gen_j:     String;
    let     c_last:    u128;
    let     out_file:  File;
    let     num_last:  u32;

    let     b:         &std::path::Path                  = std::path::Path::new(OUT_FILE_NAME);
    
    //parse cli args
    if let Some(r_seed) = std::env::args().nth(1) {
        num = r_seed.parse().expect("Invalid Seed (seed must be an unsinged int)")
    };
    
    if let Some(r_skip) = std::env::args().nth(2) { skip = r_skip.parse().expect("Invalid skip number (skip number must be an unsinged int)"); };
    if let Some(r_last) = std::env::args().nth(3) { last = r_last.parse().expect("Invalid last number (last number must be an unsinged int)"); };
    if let Some(r_zip)  = std::env::args().nth(4) { zip  = r_zip.parse::<u32>().expect("fuck"); zip_flag = true; };
     
    assert!(last > skip, "Last number must be greater than the number of skipped iterations.");
    
    set_cc_handler();
    
    let mut worker_threads:    Vec<thread::JoinHandle<()>> = Vec::new();
    
    let generator_thread:  JoinHandle<IPGenerator>;
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

    println!("[ @MAIN_THREAD      ][ Starting threads! ]");
    
    println!("[ @MAIN_THREAD      ][ Launching DisplayThread ]");
    
    display_thread   = launch_display_thread();

    generator_thread = launch_generator_thread(skip.clone(), num.clone(), last, zip, zip_flag);
    write_thread     = launch_write_thread(out_file);
    
    if cfg!(debug_assertions) { status_thread  = launch_status_thread(); };
    
    display(MessageToPrintOrigin::MainThread, "[ Launching WorkerThreads ]");
    launch_worker_threads(&mut worker_threads);
    
    let used_generator = generator_thread.join().unwrap();

    (c_last, num_last) = used_generator.gen_state();
    
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

    display(MessageToPrintOrigin::MainThread, "[ Waiting for display thread. ]");
    QUEUE_TO_PRINT.add(MessageToPrint::End);
    display_thread.join().unwrap();

    if let Some(status_thread) = status_thread {
        println!("[ @MAIN_THREAD      ][ Waiting for status thread. ]");
        status_thread.join().unwrap();
    };    
    
    
    println!("[ End. ]");
    
}
