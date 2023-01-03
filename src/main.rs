#![allow(non_snake_case)]
#![allow(clippy::needless_return)] // I like my return statements
#![allow(clippy::zero_prefixed_literal)] // I'tis a STILISTIC choise and I took that personal

use std::str::FromStr;
use std::{fs::OpenOptions, net::Ipv4Addr};
use std::sync::atomic::Ordering;
use std::thread::JoinHandle;

use std::fs::File;

use clap::Parser;

mod generators;
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

    let     numspace:           u128                                = count_posibilites(args.last - args.skip);    
    let     worker_threads:     ThreadHandler<()>                   = ThreadHandler::<()>::new();
    let     generator_thread:   ThreadHandler<IPGenerator>          = ThreadHandler::<IPGenerator>::new();
    let mut status_thread:      std::option::Option<JoinHandle<()>> = std::option::Option::None;
    let     display_thread:     JoinHandle<()>;
    let     write_thread:       JoinHandle<()>;
    let     used_generator:     IPGenerator;
    let     zip:                u128;


    assert!(args.last > args.skip, "Last number must be greater than the number of skipped iterations.");
    
    let zip: u32 = match Ipv4Addr::from_str(&args.zip) {
        Ok(zipip) => Into::<u32>::into(zipip),
        Err(_) => {
            match args.zip.parse::<u32>() {
                Ok(nzip) => { nzip },
                Err(_) => { panic!("Zip number must be either a valid IPv4 adress or a u32.") },
            }
        },
    };

    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move | panic_info | {
        orig_hook(panic_info);
        std::process::exit(1);
    }));
    
    out_file = get_outfile(&args.outfile);
    
    if args.use_zip {
        println!("{}", format!("[ @MAIN_THREAD      ][ We are zipping to {} ]", zip));
    }
    println!("{}", format!("[ @MAIN_THREAD      ][ The seed is {} ]", args.seed));
    println!("{}", format!("[ @MAIN_THREAD      ][ This run will generate {} valid IPs ]", numspace));
    
    set_cc_handler();

    println!("[ @MAIN_THREAD      ][ Starting threads! ]");
    
    println!("[ @MAIN_THREAD      ][ Launching DisplayThread ]");
    
    display_thread     = launch_display_thread();
    write_thread       = launch_write_thread(out_file);
    launch_generator_thread(generator_thread.clone(), worker_threads.clone(), args.clone(), zip);
    if args.debug_status {
        status_thread  = launch_status_thread();
    };

    launch_worker_threads(generator_thread.clone(), worker_threads.clone(), args.use_host_resolver, args.use_trust_dns, args.use_system_dns);

    println!("[ @MAIN_THREAD      ][ SYSTEMS GO SIGNAL SET! ]");
    READY___SET_GO_SIGNAL.store(true, Ordering::Relaxed);

    used_generator = generator_thread.join();
    
    worker_threads.join_all(MessageToPrintOrigin::MainThread);
    
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
        DISPLAY___STOP_SIGNAL.store(true, Ordering::Relaxed);
        status_thread.join().unwrap();
    };    
    
    
    println!("[ End. ]");
    
}
