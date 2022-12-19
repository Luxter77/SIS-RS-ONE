
use std::io;
use std::io::Write;
use std::sync::atomic::Ordering;
use std::thread;
use std::thread::JoinHandle;
use std::thread::sleep;
use std::time::Duration;

use crate::message::*;
use crate::r#static::*;

pub fn display(source: MessageToPrintOrigin, msg: &str) {
    QUEUE_TO_PRINT.add( MessageToPrint::ToDisplay(source, msg.to_owned()) )
}

fn display_status() {
    let mut stop_signal_status: [bool;  4];
    let mut queue_sizes:        [usize; 3];
    let mut last_items:         (MessageToCheck, MessageToWrite);

    loop {
        stop_signal_status = [
            GENERATOR_STOP_SIGNAL.load(Ordering::Relaxed),
            QUERYER___STOP_SIGNAL.load(Ordering::Relaxed),
            WRITER____STOP_SIGNAL.load(Ordering::Relaxed),
            DISPLAY___STOP_SIGNAL.load(Ordering::Relaxed),
        ];

        
        queue_sizes = [
            QUEUE_TO_CHECK.size(),
            QUEUE_TO_WRITE.size(),
            QUEUE_TO_WRITE.size(),
        ];

        last_items = (
            match QUEUE_TO_CHECK.peek() {
                Ok(message) => { message },
                Err(_) => { MessageToCheck::EmptyQueue },
            },
            match QUEUE_TO_WRITE.peek() {
                Ok(message) => { message },
                Err(_) => { MessageToWrite::EmptyQueue },
            },
        );
        

        QUEUE_TO_PRINT.add(MessageToPrint::ToDisplay(
            MessageToPrintOrigin::DisplayThread,
            format!("[ Signal status: {:?}; queue sizes: {:?}; last times: {:?} ]", stop_signal_status, queue_sizes, last_items)
        ));

        if DISPLAY___STOP_SIGNAL.load(Ordering::Relaxed) { break };

        sleep(Duration::from_secs_f32(0.3));
    };
}

pub(crate) fn launch_status_thread() -> Option<JoinHandle<()>> {    
    let status_thread: Option<JoinHandle<()>>;

    if cfg!(debug_assertions) {
        display(MessageToPrintOrigin::MainThread, "[ Launching StatusThread ]");
        status_thread = std::option::Option::Some(thread::Builder::new().name("StatusThread".into()).spawn(move || { display_status(); }).unwrap());
    } else {
        status_thread = std::option::Option::None;
    };

    return status_thread;
}

pub(crate) fn launch_display_thread() -> JoinHandle<()> {    
    return thread::Builder::new().name("DisplayThread".into()).spawn(move || { 
        let mut pending: bool = false;
        
        loop {
            let mut message: MessageToPrint = MessageToPrint::EmptyQueue;
            
            if QUEUE_TO_PRINT.size() < 100 {
                sleep(Duration::from_millis(2));
            };
            
            if let Ok(msg) = QUEUE_TO_PRINT.get() {
                message = msg;
                pending = true;
            };

            if pending { 
                match message {
                    MessageToPrint::ToDisplay(d_origin, message) => {
                        match d_origin {
                            MessageToPrintOrigin::GeneratorThread => println!("[ @GENERATOR_THREAD ]{}", message),
                            MessageToPrintOrigin::QueryerThread   => println!("[ @QUERYER_THREAD   ]{}", message),
                            MessageToPrintOrigin::WriterThread    => println!("[ @WRITER_THREAD    ]{}", message),
                            MessageToPrintOrigin::DisplayThread   => println!("[ @DISPLAY_THREAD   ]{}", message),
                            MessageToPrintOrigin::MainThread      => println!("[ @MAIN_THREAD      ]{}", message),
                        };
                    },
                    MessageToPrint::End => { break },
                    _ => {},
                };
            
                io::stdout().flush().expect("Unable to flush stdout!");
                
                pending = true;
            };
            
            if DISPLAY___STOP_SIGNAL.load(Ordering::Relaxed) { break };
        }
    }).unwrap();
}
