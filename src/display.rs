
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
    if cfg!(debug_assertions) {
        println!("{}", match source {
            MessageToPrintOrigin::CustomThread(src) => format!("[ @{src: >17} ]{msg}"),
            MessageToPrintOrigin::GeneratorThread   => format!("[ @GENERATOR_THREAD ]{}", msg),
            MessageToPrintOrigin::QueryerThread     => format!("[ @QUERYER_THREAD   ]{}", msg),
            MessageToPrintOrigin::WriterThread      => format!("[ @WRITER_THREAD    ]{}", msg),
            MessageToPrintOrigin::DisplayThread     => format!("[ @DISPLAY_THREAD   ]{}", msg),
            MessageToPrintOrigin::MainThread        => format!("[ @MAIN_THREAD      ]{}", msg),
        });
        
        io::stdout().flush().expect("Unable to flush stdout!");
    } else {
        QUEUE_TO_PRINT.add( MessageToPrint::ToDisplay(source, msg.to_owned()) )
    }
}

fn display_status() {
    let mut stop_signal_status:      [bool;  5];
    let mut prev_stop_signal_status: [bool;  5];
    
    let mut queue_sizes:             [usize; 3];
    let mut prev_queue_sizes:        [usize; 3];
    
    let mut last_items:              (MessageToCheck, MessageToWrite);
    let mut prev_last_items:         (MessageToCheck, MessageToWrite);

    prev_stop_signal_status = [
        READY___SET_GO_SIGNAL.load(Ordering::Relaxed),
        GENERATOR_STOP_SIGNAL.load(Ordering::Relaxed), QUERYER___STOP_SIGNAL.load(Ordering::Relaxed),
        WRITER____STOP_SIGNAL.load(Ordering::Relaxed), DISPLAY___STOP_SIGNAL.load(Ordering::Relaxed),
    ];
    
    prev_queue_sizes = [ QUEUE_TO_CHECK.size(), QUEUE_TO_WRITE.size(), QUEUE_TO_WRITE.size() ];

    prev_last_items = (
        match QUEUE_TO_CHECK.peek() { Ok(message) => { message }, Err(_) => { MessageToCheck::EmptyQueue } },
        match QUEUE_TO_WRITE.peek() { Ok(message) => { message }, Err(_) => { MessageToWrite::EmptyQueue } },
    );

    QUEUE_TO_PRINT.add(MessageToPrint::ToDisplay(MessageToPrintOrigin::DisplayThread,
        format!("[ Signal status: {:?}; queue sizes: {:?}; last times: {:?} ]", prev_stop_signal_status, prev_queue_sizes, prev_last_items)
    ));

    while !DISPLAY___STOP_SIGNAL.load(Ordering::Relaxed) || (prev_last_items == (MessageToCheck::End, MessageToWrite::EmptyQueue)) {
        stop_signal_status = [
            READY___SET_GO_SIGNAL.load(Ordering::Relaxed),
            GENERATOR_STOP_SIGNAL.load(Ordering::Relaxed), QUERYER___STOP_SIGNAL.load(Ordering::Relaxed),
            WRITER____STOP_SIGNAL.load(Ordering::Relaxed), DISPLAY___STOP_SIGNAL.load(Ordering::Relaxed),
        ];
        
        queue_sizes = [ QUEUE_TO_CHECK.size(), QUEUE_TO_WRITE.size(), QUEUE_TO_WRITE.size() ];

        last_items = (
            match QUEUE_TO_CHECK.peek() { Ok(message) => { message }, Err(_) => { MessageToCheck::EmptyQueue } },
            match QUEUE_TO_WRITE.peek() { Ok(message) => { message }, Err(_) => { MessageToWrite::EmptyQueue } },
        );
        
        if (stop_signal_status != prev_stop_signal_status) || (queue_sizes != prev_queue_sizes) || (last_items != prev_last_items) {
            QUEUE_TO_PRINT.add(MessageToPrint::ToDisplay(
                MessageToPrintOrigin::DisplayThread,
                format!("[ Signal status: {:?}; queue sizes: {:?}; last times: {:?} ]", stop_signal_status, queue_sizes, last_items)
            ));
        }

        prev_stop_signal_status = stop_signal_status;
        prev_queue_sizes        = queue_sizes;
        prev_last_items         = last_items;

        sleep(Duration::from_secs_f32(0.3));
    };
}

pub(crate) fn launch_status_thread() -> Option<JoinHandle<()>> {
    println!("{}", "[ @MAIN_THREAD      ][ Launching StatusThread ]");   
    return std::option::Option::Some(thread::Builder::new().name("StatusThread".into()).spawn(move || { display_status(); }).unwrap());
}

pub(crate) fn launch_display_thread() -> JoinHandle<()> {    
    println!("{}", "[ @MAIN_THREAD      ][ Launching DisplayThread ]");
    return thread::Builder::new().name("DisplayThread".into()).spawn(move || { 
        let mut pending: bool = false;
        
        
        while !DISPLAY___STOP_SIGNAL.load(Ordering::Relaxed) {
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
                        println!("{}", match d_origin {
                            MessageToPrintOrigin::CustomThread(src) => format!("[ @{src: >17} ]{message}"),
                            MessageToPrintOrigin::GeneratorThread   => format!("[ @GENERATOR_THREAD ]{message}"),
                            MessageToPrintOrigin::QueryerThread     => format!("[ @QUERYER_THREAD   ]{message}"),
                            MessageToPrintOrigin::WriterThread      => format!("[ @WRITER_THREAD    ]{message}"),
                            MessageToPrintOrigin::DisplayThread     => format!("[ @DISPLAY_THREAD   ]{message}"),
                            MessageToPrintOrigin::MainThread        => format!("[ @MAIN_THREAD      ]{message}"),
                        });
                    },
                    MessageToPrint::Wait(time) => { sleep(time) },
                    MessageToPrint::End => { break },
                    _ => {},
                };
            
                io::stdout().flush().expect("Unable to flush stdout!");
                
                pending = true;
            };
        };
    }).unwrap();
}
