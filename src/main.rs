#![allow(non_snake_case)]

use std::fs::OpenOptions;
use std::sync::atomic::Ordering;
use std::thread::JoinHandle;

use std::fs::File;
use std::{thread, u128};

use clap::Parser;

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
    println!("{}", match CTL_C_C___STOP_SIGNAL.get() {
        0 => { GENERATOR_STOP_SIGNAL.store(true, Ordering::Relaxed); "[ @CT_C_SIG_HANDLER ][ Keyboard Interrupt recieved, signaling generator thread to stop. ]" },
        1 => { QUERYER___STOP_SIGNAL.store(true, Ordering::Relaxed); "[ @CT_C_SIG_HANDLER ][ Keyboard Interrupt recieved, signaling query threads to stop. ]" },
        2 => { WRITER____STOP_SIGNAL.store(true, Ordering::Relaxed); "[ @CT_C_SIG_HANDLER ][ Keyboard Interrupt recieved, signaling writer thread to stop! ]" },
        3 => { DISPLAY___STOP_SIGNAL.store(true, Ordering::Relaxed); "[ @CT_C_SIG_HANDLER ][ Keyboard Interrupt recieved, signaling display thread to stop! ]" },
        _ => { "[ @CT_C_SIG_HANDLER ][ Keyboard Interrupt recieved, signaling no one, lol. ]" },
    });

    // now with 100% less integer overflow
    CTL_C_C___STOP_SIGNAL.add_one();
}

fn set_cc_handler() {
    ctrlc::set_handler( move || { ctrl_c_handler(); }).expect("Error setting Ctrl-C handler");
}

#[inline(always)]
fn get_outfile(fpath: &str) -> File {
    let out_file: File;
    let out_path: &std::path::Path = std::path::Path::new(fpath);
    if out_path.exists() {
        out_file = OpenOptions::new().append(true).open(out_path.clone()).expect("Could not open existing file!");
        println!("{}", format!("[ @MAIN_THREAD      ][ Using existing file: {} ]", out_path.display()));
    } else {
        out_file = OpenOptions::new().create(true).write(true).open(out_path.clone()).expect("Could not create output file!");
        println!("{}", format!("[ @MAIN_THREAD      ][ Created new file: {} ]", out_path.display()));
        QUEUE_TO_WRITE.add(MessageToWrite::ToWrite(String::from("IP"), String::from("HOSTNAME")));
    };
    return out_file
}

#[allow(unused_variables)]
fn main() {
    let     args:      CommandLineArguments = CommandLineArguments::parse().seed(); 
    
    let     c_last:    u128;
    let     out_file:  File;
    let     num_last:  u32;

    let     numspace:           u128 = count_posibilites(args.last - args.skip);    
    let mut worker_threads:     Vec<thread::JoinHandle<()>> = Vec::new();
    let     generator_thread:   JoinHandle<IPGenerator>;
    let     display_thread:     JoinHandle<()>;
    let mut status_thread:      std::option::Option<JoinHandle<()>> = std::option::Option::None;
    let     write_thread:       JoinHandle<()>;
    let     used_generator:     IPGenerator;

    assert!(args.last > args.skip, "Last number must be greater than the number of skipped iterations.");
    
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move | panic_info | {
        orig_hook(panic_info);
        std::process::exit(1);
    }));
    
    out_file = get_outfile(&args.outfile);
    
    println!("{}", format!("[ @MAIN_THREAD      ][ The seed is {} ]", args.seed));
    println!("{}", format!("[ @MAIN_THREAD      ][ This run will generate {} valid IPs ]", numspace));
    
    set_cc_handler();

    println!("[ @MAIN_THREAD      ][ Starting threads! ]");
    
    println!("[ @MAIN_THREAD      ][ Launching DisplayThread ]");
    
    display_thread     = launch_display_thread();
    generator_thread   = launch_generator_thread(args.skip, args.seed, args.last, args.zip, args.use_zip, args.no_continue, args.generator_strategy);
    write_thread       = launch_write_thread(out_file);
    if args.debug_status {
        status_thread  = launch_status_thread();
    };
    launch_worker_threads(&mut worker_threads, args.use_host_resolver, args.use_trust_dns, args.use_system_dns);

    used_generator = generator_thread.join().unwrap();
    
    display(MessageToPrintOrigin::MainThread, "[ waiting for worker threads ]");
    while let Some(cur_thread) = worker_threads.pop() {
        if cfg!(debug_assertions) { display(MessageToPrintOrigin::MainThread, &format!("[ waiting for worker thread: {:?} ]", cur_thread.thread().id())); };
        cur_thread.join().unwrap();
    };
    
    QUEUE_TO_WRITE.add( MessageToWrite::End );
    
    display(MessageToPrintOrigin::MainThread, "[ Waiting for writer thread. ]");
    
    write_thread.join().unwrap();
    
    (c_last, num_last) = used_generator.gen_state();

    display(MessageToPrintOrigin::MainThread, "[ - - - - - - - - - - - - - ]");
    display(MessageToPrintOrigin::MainThread, &format!("[ We started @ {} Iterations ]",            args.skip));
    display(MessageToPrintOrigin::MainThread, &format!("[ The last number was => {} ]",             c_last));
    display(MessageToPrintOrigin::MainThread, &format!("[ We found {} records ({} idstinct IPs out of {} in usable space) ]", FOUND_COUNT.get(), FOUND_DISTINCT_COUNT.get(), numspace));
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
