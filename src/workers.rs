use std::io::Write;

use crate::{
    generators::generator::generate,
    generators::IPGenerator,
    resolv::resolv_worker,
    display::display,
    r#static::*,
    message::*,
};

pub(crate) fn write_worker(mut out_file: std::fs::File) {
    while !WRITER____STOP_SIGNAL.load(std::sync::atomic::Ordering::Relaxed) {
        if let Ok( message ) = QUEUE_TO_WRITE.get() {
            match message {
                MessageToWrite::ToWrite(ip, host) => {
                    writeln!(&mut out_file, "{a}, {b}", a=ip, b=host).expect("Can't write to out file!");
                },
                MessageToWrite::End => { break },
                MessageToWrite::EmptyQueue => todo!(),
            };
        } else {
            std::thread::sleep(std::time::Duration::from_millis(SLEEP_TIME * 10));
        };
    };
    display(MessageToPrintOrigin::Writer, "[ Write End ]");
}

pub(crate) fn launch_generator_thread(mut generator_handle: ThreadHandler<IPGenerator>, worker_handles: ThreadHandler<()>, args: CommandLineArguments, zip: u32) {
    display(MessageToPrintOrigin::Main, "[ Launching GeneratorThread ]");
    generator_handle.add(std::thread::Builder::new().name("GeneratorThread".into()).spawn(move || { return generate(worker_handles, args, zip); }).unwrap());
}

pub(crate) fn launch_write_thread(out_file: std::fs::File) -> std::thread::JoinHandle<()> {
    display(MessageToPrintOrigin::Main, "[ Launching WriterThread ]");
    return std::thread::Builder::new().name("WriterThread".into()).spawn(move || { write_worker(out_file); }).unwrap();
}

pub(crate) fn launch_worker_threads(generator_handle: ThreadHandler<IPGenerator>, mut worker_handles: ThreadHandler<()>, t_use_host_resolver: bool, t_use_trust_dns: bool, t_use_system_dns: bool) {
    display(MessageToPrintOrigin::Main, "[ Launching WorkerThreads ]");
    for n in 0..(CORES * 4) { // Starts query worker threads
        let (nam, gen_threads): (String, ThreadHandler<IPGenerator>) = (format!("QueryerThread#{}", n), generator_handle.clone());
        if cfg!(debug_assertions) { display(MessageToPrintOrigin::Main, &format!("[ Launching: {} ]", nam.clone())); };
        worker_handles.add(std::thread::Builder::new().name(nam).spawn(move || { resolv_worker(t_use_host_resolver, t_use_trust_dns, t_use_system_dns, gen_threads); }).unwrap());
    };
}
