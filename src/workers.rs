use std::io::Write;

use crate::{
    generators::IPGenerator,
    resolv::resolv_worker,
    generator::generate,
    display::display,
    r#static::*,
    message::*,
};

pub(crate) fn write_worker(mut out_file: std::fs::File) {
    loop {
        if let Ok( message ) = QUEUE_TO_WRITE.get() {
            match message {
                MessageToWrite::ToWrite(ip, host) => {
                    // writeln!(&mut out_file, "{a}, {b}", a=ip, b=host).expect("Can't write to out file!");
                },
                MessageToWrite::End => { break },
                MessageToWrite::EmptyQueue => todo!(),
            };
        } else {
            if WRITER____STOP_SIGNAL.load(std::sync::atomic::Ordering::Relaxed) { break };
            std::thread::sleep(std::time::Duration::from_millis(SLEEP_TIME * 10));
        };
    };
    display(MessageToPrintOrigin::WriterThread, "[ Write End ]");
}

pub(crate) fn launch_generator_thread(skip: u128, num: u128, last: u128, zip: u32, zip_flag: bool) -> std::thread::JoinHandle<IPGenerator> {
    display(MessageToPrintOrigin::MainThread, "[ Launching GeneratorThread ]");
    return std::thread::Builder::new().name("GeneratorThread".into()).spawn(move || { return generate(skip, num, last, zip, zip_flag); }).unwrap();
}

pub(crate) fn launch_write_thread(out_file: std::fs::File) -> std::thread::JoinHandle<()> {
    display(MessageToPrintOrigin::MainThread, "[ Launching WriterThread ]");
    return std::thread::Builder::new().name("WriterThread".into()).spawn(move || { write_worker(out_file); }).unwrap();
}

pub(crate) fn launch_worker_threads(worker_threads: &mut Vec<std::thread::JoinHandle<()>>) {
    for n in 0..(CORES * 4) { // Starts query worker threads
        let nam = format!("QueryerThread#{}", n);
        if cfg!(debug_assertions) { display(MessageToPrintOrigin::MainThread, &format!("[ Launching: {} ]", nam.clone())); };
        worker_threads.push(std::thread::Builder::new().name(nam).spawn(move || { resolv_worker(); }).unwrap());
    };
}
