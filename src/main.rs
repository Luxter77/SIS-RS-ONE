#![allow(unreachable_code)]
#![allow(non_snake_case)]

use std::thread::{sleep, JoinHandle};
use std::sync::{Mutex, Arc};
use std::fs::OpenOptions;
use std::time::Duration;

#[allow(unused_imports)]
use std::net::{SocketAddr, Ipv4Addr, IpAddr};

use std::{thread, vec, u128};
use std::io::Write;
use std::fs::File;
use std::io;

#[allow(unused_imports)]
#[cfg(feature = "trust-dns")]
use trust_dns_resolver::{Resolver, config::{ResolverConfig, ResolverOpts, NameServerConfig, Protocol}};

#[allow(unused_imports)]
use human_sort::compare;

use queues::{Queue, IsQueue};
use pad::{PadStr, Alignment};
use dns_lookup::lookup_addr;
use num_bigint::BigUint;
use rand::Rng;

static MAX_IIP:    u128 = 4294967295u128; // 255.255.255.255

static NEXT_PRIME: u128 = 4294967311u128; // 4294967295 is the next prime to 4294967295 (MAX_IIP)
static LAST_NUMBR: u128 = NEXT_PRIME + 1u128; // 4294967295 + 1

static A_PRIMA: u128 = 273u128;
static C_PRIMA: u128 = 2147483655u128;
static M_PRIMA: u128 = LAST_NUMBR;

static CORES:       usize = 20;
static QUEUE_LIMIT: usize = CORES * 5;

static OUT_FILE_NAME: &str = "RESOLVED.csv";

static SLEEP_TIME: u64 = 10;

static USE_SYSTEM_DNS: bool = !(cfg!(feature = "trust-dns"));

#[allow(dead_code)]
#[derive(Clone)]
#[derive(Debug)]
enum MessageToCheck {
    EmptyQueue,
    ToCheck(u128, BigUint),
    End,
}

#[allow(dead_code)]
#[derive(Clone)]
#[derive(Debug)]
enum MessageToWrite {
    EmptyQueue,
    ToWrite(String, String),
    End,
}


// Counts how many posible distinct numbers can this program generate (using current filters)
pub fn count_posibilites() -> u128 {
    let mut count: u128 = 0;
    for (s, e) in RESERVED_RANGES {
        count += e - s;
    };
    count -= NEXT_PRIME;

    println!("{}", count);

    return count;
}
    
fn check_reserved(num: BigUint) -> bool {
    if num > BigUint::from(MAX_IIP) {
        return false;
    };

    for (start, end) in RESERVED_RANGES {
        if (BigUint::from(start) <= num) && (num <= BigUint::from(end)) {
            return false;
        };
    };

    return true;
}

fn trust_dns_lookup_addr(lipn: &mut Vec<String>, ip: &Ipv4Addr, resolver: &Resolver) {
    if let Ok(res) = resolver.reverse_lookup(std::net::IpAddr::V4(ip.to_owned())) {
        #[cfg(debug_assertions)] {
            let ips: Vec<String> = res.iter().map( |nam| -> String { nam.to_ascii() } ).collect();
            if ips.len() > 1 { println!("{}", format!("IP HAS MORE THAN ONE ADRESS! -> {:?}", ips)); };
            lipn.extend(ips.iter().map( move | nam: &String | nam.to_owned() ).collect::<std::collections::HashSet<_>>());
        }
        #[cfg(not(debug_assertions))] {
            lipn.extend(res.iter().map( |nam| -> String { nam.to_ascii() } ).collect::<std::collections::HashSet<_>>());
        }
        #[cfg(feature = "host-resolv")]  {
            if lipn.len() > 0 {
                let mut h_res_conf = ResolverConfig::new();          
                h_res_conf.add_name_server(NameServerConfig::new(SocketAddr::new(IpAddr::V4(ip.clone()), 53), Protocol::default()));
                if let Ok(h_res) = Resolver::new(h_res_conf, ResolverOpts::default()).unwrap().reverse_lookup(std::net::IpAddr::V4(Ipv4Addr::from(iip.to_string().parse::<u32>().unwrap()))) {
                    lipn.extend(h_res.iter().map( |nam| -> String { nam.to_ascii() } ).collect::<std::collections::HashSet<_>>());
                };
            };
        };
    };
}
    


fn check_worker(queue: Arc<Mutex<Queue<MessageToCheck>>>, out_queue: Arc<Mutex<Queue<MessageToWrite>>>, stop_sig: Arc<Mutex<Vec<bool>>>) {
    let mut pending: bool = false;

    // logic too deepth for the compiler?
    // This will never get read, but the all knowing compiler insists...
    let mut iip:     BigUint = BigUint::from(0u128); 
    let mut c:       u128    =               0u128;

    let mut p:       f32;

    #[cfg(feature = "trust-dns")]
    let resolver: trust_dns_resolver::Resolver = trust_dns_resolver::Resolver::default().unwrap();

    loop {
        if stop_sig.lock().unwrap()[0] { break };
        
        if let Ok( MessageToCheck::End ) = queue.lock().unwrap().peek() { break };
        
        if let Ok( MessageToCheck::ToCheck(p_c, p_iip) ) = queue.lock().unwrap().remove() {
            (c, iip, pending) = (p_c.clone(), p_iip.clone(), true);
        };

        if pending {
            p  = (c as f32) * 100.0f32 / (LAST_NUMBR as f32);

            if check_reserved(iip.clone()) {
                let mut lipn:   Vec<String> = Vec::new();
                
                let     ip:     Ipv4Addr    = Ipv4Addr::from(iip.to_string().parse::<u32>().unwrap());
                let     sip:    String      = ip.clone().to_string().pad_to_width_with_alignment(15, Alignment::Right);

                if USE_SYSTEM_DNS {
                    lipn.push(lookup_addr(&ip.into()).unwrap());
                } else {
                    #[cfg(feature = "trust-dns")]
                    trust_dns_lookup_addr(&mut lipn, &ip, &resolver);
                };
                
                lipn.sort_by(| a, b | human_sort::compare(a.as_str(), b.as_str()));
                
                for ipn in lipn {
                    if ipn != ip.to_string() {
                        println!("{}", format!("[{p:>17}%][{a:>10}/{t}][IP: {b:>15}][DNS: {d}]", a=c, p=p, t=LAST_NUMBR, b=sip, d=ipn));
                        out_queue.lock().unwrap().add(MessageToWrite::ToWrite(ip.to_string(), ipn) ).unwrap();
                    } else {
                        #[cfg(debug_assertions)]
                        println!("{}", format!("[{p:>17}%][{a:>10}/{t}][IP: {b:>15}][IPN: {d}]", a=c, p=p, t=LAST_NUMBR, b=sip, d=ipn));
                    };
                };
            } else {
                // #[cfg(debug_assertions)]
                // println!("{}", format!("[{p:>17}%][{a:>10}/{t}][IP: {b:>15}][MSG: REJECTED!]", a=c, p=p, t=LAST_NUMBR, b=iip.clone().to_string().pad_to_width_with_alignment(15, Alignment::Right)));
            };
            io::stdout().flush().expect("\n\rUnable to flush stdout!");
            pending = false;
        } else {
            sleep(Duration::from_millis(SLEEP_TIME * 3));
        };

        {
            #[cfg(debug_assertions)]
            println!("{}", format!("to_write queue size is currently: {} items long.", queue.lock().unwrap().size()));
        };
    };
}

fn write_worker(mut out_file: File, out_queue: Arc<Mutex<Queue<MessageToWrite>>>, stop_sig: Arc<Mutex<Vec<bool>>>) {
    loop {
        if let Ok( message ) = out_queue.lock().unwrap().remove() {
            match message {
                MessageToWrite::ToWrite(ip, host) => {
                    writeln!(&mut out_file, "{a}, {b}", a=ip, b=host).expect("Can't write to out file!");
                },
                MessageToWrite::End => { break },
                MessageToWrite::EmptyQueue => todo!(),
            }
        } else {
            if stop_sig.lock().unwrap()[0] { break };
            sleep(Duration::from_millis(SLEEP_TIME * 10));
        };
    };
}

fn generate(generator_stop_signal: Arc<Mutex<Vec<bool>>>, to_check: Arc<Mutex<Queue<MessageToCheck>>>, mut skip: BigUint, mut num: BigUint, last: BigUint) {
    let mut c: u128 = 0;
    
    let first_number: BigUint = num.clone();

    loop { // Generates IIPs for the query worker threads
        if generator_stop_signal.lock().unwrap()[0] { break };

        let can_go: bool = to_check.lock().unwrap().size() < QUEUE_LIMIT * 10;

        if can_go {
            c += 1;
            
            if skip == BigUint::from(0u128) {
                to_check.lock().unwrap().add( MessageToCheck::ToCheck(c, num.clone()) ).unwrap();
            } else { 
                skip -= BigUint::from(1u128);
            }

            num = (BigUint::from(A_PRIMA) * num + BigUint::from(C_PRIMA)) % BigUint::from(M_PRIMA);
            
            if num == first_number {
                println!("We went all the way arround!!!1!!11!1one!!1!111");
                break;
            };
            
            if num == last {
                println!("We reached the stipulated end!");
                break;
            }
            
            {           
                #[cfg(debug_assertions)]
                println!("{}", format!("to_check queue size is currently: {} items long.", to_check.lock().unwrap().size()));
            };
        } else {
            sleep(Duration::from_secs(SLEEP_TIME / 2));
        };
    };

    println!("{}", format!("The last number was => {}\nIt appeared after {} iterations.", num, c));

    to_check.lock().unwrap().add( MessageToCheck::End ).unwrap();
}

fn display_update(display_stop_signal: Arc<Mutex<Vec<bool>>>) {
    loop {
        if display_stop_signal.lock().unwrap()[0] { break }
        sleep(Duration::from_secs_f32(0.1));
        io::stdout().flush().expect("\n\rUnable to flush stdout!");
    }
}

#[cfg(debug_assertions)]
fn display_status(display_stop_signal: Arc<Mutex<Vec<bool>>>, generator_stop_signal: Arc<Mutex<Vec<bool>>>, queryer_stop_signal: Arc<Mutex<Vec<bool>>>, writer_stop_signal: Arc<Mutex<Vec<bool>>>, to_write: Arc<Mutex<Queue<MessageToWrite>>>, to_check: Arc<Mutex<Queue<MessageToCheck>>>) {
    
    let mut stop_signal_status: [bool;  3];
    let mut queue_sizes:        [usize; 2];
    let mut last_items:         (MessageToCheck, MessageToWrite);


    loop {
        {
            stop_signal_status = [
                generator_stop_signal.lock().unwrap()[0],
                queryer_stop_signal.lock().unwrap()[0],
                writer_stop_signal.lock().unwrap()[0],
            ];
        };

        {
            let inquee = to_check.lock().unwrap(); 
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

        if display_stop_signal.lock().unwrap()[0] { break };

        sleep(Duration::from_secs_f32(0.3));
    };
}

fn ctrl_c_handler(cc_counter: Arc<Mutex<Vec<u8>>>, cc_generator_stop_signal: Arc<Mutex<Vec<bool>>>, cc_queryer_stop_signal: Arc<Mutex<Vec<bool>>>, cc_writer_stop_signal: Arc<Mutex<Vec<bool>>>, cc_display_stop_signal: Arc<Mutex<Vec<bool>>>) {
    let n = cc_counter.lock().unwrap()[0];
            
    println!("{}", match n {
        0 => {
            cc_generator_stop_signal.lock().unwrap()[0] = true;
            "Keyboard Interrupt recieved, signaling generator thread to stop."
        },
        1 => {
            cc_queryer_stop_signal.lock().unwrap()[0]   = true;                    
            "Keyboard Interrupt recieved, signaling query threads to stop."
        },
        2 => {
            cc_writer_stop_signal.lock().unwrap()[0]    = true;
            "Keyboard Interrupt recieved, signaling writer thread to stop!"
        },
        3 => {
            cc_display_stop_signal.lock().unwrap()[0]   = true;
            "Keyboard Interrupt recieved, signaling display thread to stop!"
        },
        _ => {                
            "Keyboard Interrupt recieved, signaling no one, lol."
        },
    } );            

    cc_counter.lock().unwrap()[0] = n + 1;
}

#[allow(unused_variables)]
fn main() {
    let mut num:       BigUint                           = BigUint::from(rand::thread_rng().gen::<u128>());

    let mut skip:      BigUint                           = BigUint::from(0u128);
    
    let mut last:      BigUint                           = BigUint::from(LAST_NUMBR);
    
    let     out_file:  File;
    
    let     b:         &std::path::Path                  = std::path::Path::new(OUT_FILE_NAME);
    
    let     to_write:  Arc<Mutex<Queue<MessageToWrite>>> = Arc::new(Mutex::new(Queue::new()));
    let     to_check:  Arc<Mutex<Queue<MessageToCheck>>> = Arc::new(Mutex::new(Queue::new()));

    let     generator_stop_signal:  Arc<Mutex<Vec<bool>>> = Arc::new(Mutex::new(vec![false]));
    let     queryer_stop_signal:    Arc<Mutex<Vec<bool>>> = Arc::new(Mutex::new(vec![false]));
    let     writer_stop_signal:     Arc<Mutex<Vec<bool>>> = Arc::new(Mutex::new(vec![false]));
    let     display_stop_signal:    Arc<Mutex<Vec<bool>>> = Arc::new(Mutex::new(vec![false]));

    //parse cli args
    if let Some(r_seed) = std::env::args().nth(1) {
        num = r_seed.parse().expect("Invalid Seed (seed must be an unsinged int)")
    };

    if let Some(r_skip) = std::env::args().nth(2) {
        skip = r_skip.parse().expect("Invalid skip number (skip number must be an unsinged int)");
    };

    if let Some(r_last) = std::env::args().nth(3) {
        last = r_last.parse().expect("Invalid last number (last number must be an unsinged int)");
    };

    assert!(last > skip, "Last number must be greater than the number of skipped iterations.");

    {
        let cc_ctr: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(vec![0u8]));
        
        let cc_gen_st_sg: Arc<Mutex<Vec<bool>>> = generator_stop_signal.clone(); 
        let cc_qry_st_sg: Arc<Mutex<Vec<bool>>> = queryer_stop_signal.clone(); 
        let cc_wrt_st_sg: Arc<Mutex<Vec<bool>>> = writer_stop_signal.clone();
        let cc_dsp_st_sg: Arc<Mutex<Vec<bool>>> = display_stop_signal.clone();
        
        ctrlc::set_handler( move || {
            ctrl_c_handler(
                cc_ctr.clone(),
                cc_gen_st_sg.clone(),
                cc_qry_st_sg.clone(),
                cc_wrt_st_sg.clone(),
                cc_dsp_st_sg.clone(),
            );
        }).expect("Error setting Ctrl-C handler");
    };
    
    let mut worker_threads:    Vec<thread::JoinHandle<()>> = Vec::new();
    
    let generator_thread:  JoinHandle<()>;
    let display_thread:    JoinHandle<()>;
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

    println!("Starting threads!");
    
    { // Starts write worker thread
        let (queuee, sig) = (to_write.clone(), writer_stop_signal.clone());
        write_thread = thread::Builder::new().name("WriterThread".into()).spawn(move || {
            sleep(Duration::from_secs(3));
            write_worker(out_file, queuee, sig);
        }).unwrap();
    };

    for n in 0..(CORES * 4) { // Starts query worker threads
        let (_tc, oq_, ss_) = (to_check.clone(), to_write.clone(), queryer_stop_signal.clone());
        worker_threads.push(thread::Builder::new().name(format!("QueryerThread#{}", n)).spawn(move || {
            sleep(Duration::from_secs(2));
            check_worker(_tc, oq_, ss_);
        }).unwrap());
    };
    
    num = (BigUint::from(A_PRIMA) * num + BigUint::from(C_PRIMA)) % BigUint::from(M_PRIMA);

    println!("{}", format!("first number is: {}", num.clone()));

    {
        let generator_stop_signal: Arc<Mutex<Vec<bool>>> = generator_stop_signal.clone();
        let to_check: Arc<Mutex<Queue<MessageToCheck>>>  = to_check.clone();
        let skip: BigUint                                = skip.clone();
        let num: BigUint                                 = num.clone();

        generator_thread = thread::Builder::new().name("GeneratorThread".into()).spawn(move || {
            sleep(Duration::from_millis(SLEEP_TIME));
            generate(generator_stop_signal, to_check, skip, num, last);
        }).unwrap();
    };

    {
        let d_generator_stop_signal: Arc<Mutex<Vec<bool>>> = generator_stop_signal.clone();
        let d_queryer_stop_signal:   Arc<Mutex<Vec<bool>>> = queryer_stop_signal.clone();
        let d_writer_stop_signal:    Arc<Mutex<Vec<bool>>> = writer_stop_signal.clone();
        
        #[cfg(debug_assertions)]
        let display_stop_signal:     Arc<Mutex<Vec<bool>>> = display_stop_signal.clone();

        let display_stop_signal2:    Arc<Mutex<Vec<bool>>> = display_stop_signal.clone();
        
        let d_to_write:              Arc<Mutex<Queue<MessageToWrite>>> = to_write.clone();
        let d_to_check:              Arc<Mutex<Queue<MessageToCheck>>> = to_check.clone();

        
        #[cfg(debug_assertions)]
        {
            display_thread = thread::Builder::new().name("DisplayThread".into()).spawn(move || {
                display_status(display_stop_signal, d_generator_stop_signal, d_queryer_stop_signal, d_writer_stop_signal, d_to_write, d_to_check);
            }).unwrap();
        };

        thread::Builder::new().name("DisplayUpdateThread".into()).spawn(move || {
            display_update(display_stop_signal2);
        }).unwrap();
    };

    generator_thread.join().unwrap();

    {
        #[cfg(debug_assertions)]
        println!("We got hereeeeeeeeee");
    };

    while let Some(cur_thread) = worker_threads.pop() {
        {
            #[cfg(debug_assertions)]
            println!("{}", format!("waiting for worker thread: {:?}.", cur_thread.thread().id()));
        };
        cur_thread.join().unwrap();
    };

    to_write.lock().unwrap().add( MessageToWrite::End ).unwrap();
    
    {
        #[cfg(debug_assertions)]
        println!("waiting for writer thread.");
    };

    write_thread.join().unwrap();

    display_stop_signal.lock().unwrap()[0] = true;
    
    {
        #[cfg(debug_assertions)]
        display_thread.join().unwrap();
    };

    println!("End.");

}

// from https://github.com/robertdavidgraham/masscan/blob/master/data/exclude.conf
// and others, we really dont want these to be angry at us...
// also private networks lol
static RESERVED_RANGES: [(u128, u128); 334] = [
    (0,          16777215 ),
    (70633728,   70633983 ),
    (93893376,   93893631 ),
    (100663296,  117440511),
    (117440512,  134217727),
    (135045632,  135046399),
    (135156736,  135157759),
    (135172352,  135173119),
    (135395840,  135396607),
    (167772160,  184549375),
    (184549376,  201326591),
    (352321536,  369098751),
    (369098752,  385875967),
    (387645440,  387710975),
    (401047552,  401080319),
    (436207616,  452984831),
    (469762048,  486539263),
    (486539264,  503316479),
    (503316480,  520093695),
    (521732096,  521732607),
    (521732608,  521733119),
    (521733120,  521734143),
    (553648128,  570425343),
    (625504256,  625506303),
    (625519616,  625520127),
    (642304000,  642305023),
    (788449280,  788451327),
    (845004800,  845006335),
    (846430208,  846434303),
    (846528512,  846561279),
    (846626816,  846659583),
    (922746880,  939524095),
    (1066262016, 1066262271),
    (1077869824, 1077870079),
    (1079795712, 1079803903),
    (1083264768, 1083265023),
    (1083265536, 1083266047),
    (1084133888, 1084134399),
    (1093736448, 1093736703),
    (1093754112, 1093754367),
    (1101185024, 1101186047),
    (1112514560, 1112522751),
    (1117830912, 1117831167),
    (1145331712, 1145335807),
    (1160658944, 1160667135),
    (1169182720, 1169186815),
    (1208832000, 1208836095),
    (1211386880, 1211387135),
    (1246898944, 1246899199),
    (1246928896, 1246937087),
    (1249007616, 1249008639),
    (1249050624, 1249050879),
    (1249051136, 1249051391),
    (1249051648, 1249051903),
    (1249534976, 1249535999),
    (1266614272, 1266614527),
    (1364656128, 1364721663),
    (1426866176, 1426882559),
    (1506791424, 1506793471),
    (1559617536, 1559625727),
    (1681915904, 1686110207),
    (1744526080, 1744526335),
    (1823154176, 1823154431),
    (1823156736, 1823156991),
    (1823159296, 1823159551),
    (1823161856, 1823162111),
    (1992379904, 1992380415),
    (2130706432, 2147483647),
    (2148532224, 2148597759),
    (2150105088, 2150170623),
    (2150170624, 2150187007),
    (2153119744, 2153185279),
    (2162688000, 2162753535),
    (2163212288, 2163277823),
    (2163408896, 2163474431),
    (2164981760, 2165047295),
    (2165047296, 2165112831),
    (2166292480, 2166358015),
    (2168651776, 2168717311),
    (2172321792, 2172387327),
    (2175336448, 2175401983),
    (2178351104, 2178416639),
    (2179596288, 2179661823),
    (2186805248, 2186870783),
    (2187137024, 2187137535),
    (2191458304, 2191523839),
    (2194735104, 2194800639),
    (2197159936, 2197225471),
    (2205089792, 2205155327),
    (2212691968, 2212757503),
    (2212954112, 2213019647),
    (2214264832, 2214330367),
    (2228095232, 2228095487),
    (2228124416, 2228124671),
    (2228124928, 2228125183),
    (2250506240, 2250571775),
    (2253586432, 2253651967),
    (2258042880, 2258108415),
    (2262499328, 2262564863),
    (2262564864, 2262630399),
    (2262892544, 2262958079),
    (2281701376, 2281766911),
    (2291400704, 2291466239),
    (2291924992, 2291990527),
    (2301362176, 2301427711),
    (2301755392, 2301820927),
    (2303262720, 2303328255),
    (2305556480, 2305622015),
    (2311258112, 2311323647),
    (2313027584, 2313093119),
    (2315059200, 2315124735),
    (2317746176, 2317811711),
    (2317877248, 2317942783),
    (2331639808, 2331770879),
    (2331836416, 2331901951),
    (2340749312, 2340814847),
    (2342060032, 2342125567),
    (2342912000, 2342977535),
    (2344091648, 2344157183),
    (2346582016, 2346647551),
    (2355167232, 2355232767),
    (2376269824, 2376335359),
    (2376744960, 2376753151),
    (2376753152, 2376754175),
    (2376754176, 2376754687),
    (2381381632, 2381447167),
    (2389639168, 2389704703),
    (2398879744, 2398945279),
    (2402549760, 2402680831),
    (2406809600, 2406875135),
    (2410086400, 2410151935),
    (2412904448, 2412969983),
    (2414477312, 2414542847),
    (2418016256, 2418081791),
    (2418475008, 2418540543),
    (2421293056, 2421358591),
    (2424045568, 2424111103),
    (2427256832, 2427322367),
    (2454861661, 2454861661),
    (2455175168, 2455240703),
    (2455830528, 2455896063),
    (2460549120, 2460614655),
    (2461007872, 2461073407),
    (2461204480, 2461270015),
    (2461990912, 2462056447),
    (2464350208, 2464415743),
    (2475622400, 2475687935),
    (2478571520, 2478637055),
    (2479161344, 2479226879),
    (2488205312, 2488270847),
    (2488795136, 2488860671),
    (2495938560, 2496004095),
    (2503378944, 2503380991),
    (2503383040, 2503385087),
    (2509963264, 2510028799),
    (2510946304, 2511011839),
    (2529951744, 2530017279),
    (2554789888, 2554855423),
    (2555248640, 2555314175),
    (2557018112, 2557083647),
    (2567634944, 2567700479),
    (2613444608, 2613510143),
    (2616524800, 2616590335),
    (2643197952, 2643263487),
    (2648965120, 2649030655),
    (2656960512, 2657026047),
    (2658992128, 2659057663),
    (2660171776, 2660237311),
    (2665414656, 2665480191),
    (2673246208, 2673262591),
    (2673606656, 2673672191),
    (2684682240, 2684747775),
    (2684944384, 2685009919),
    (2705915904, 2705981439),
    (2705981440, 2706046975),
    (2706112512, 2706178047),
    (2708471808, 2708537343),
    (2734751744, 2734817279),
    (2742484992, 2742550527),
    (2745171968, 2745237503),
    (2745630720, 2745696255),
    (2752184320, 2752249855),
    (2778726400, 2778791935),
    (2790785024, 2790850559),
    (2851995648, 2852061183),
    (2886729728, 2887778303),
    (2902196224, 2902261759),
    (2918531072, 2918539263),
    (2918564352, 2918564863),
    (2918571008, 2918572031),
    (2919022592, 2919038975),
    (2987528192, 2987529215),
    (2987530752, 2987531775),
    (3082163712, 3082163967),
    (3109267456, 3109268479),
    (3221225472, 3221225727),
    (3221225642, 3221225642),
    (3221225643, 3221225643),
    (3221225984, 3221226239),
    (3222030336, 3222030591),
    (3222455040, 3222455295),
    (3223563264, 3223563519),
    (3223939072, 3223941119),
    (3223941120, 3223945215),
    (3223945216, 3223946239),
    (3225721088, 3225721343),
    (3226207744, 3226208255),
    (3226208256, 3226210303),
    (3226210304, 3226214399),
    (3226214400, 3226215423),
    (3226638592, 3226638847),
    (3226731776, 3226732031),
    (3226749696, 3226749951),
    (3226749952, 3226750975),
    (3226750976, 3226751999),
    (3226784768, 3226785023),
    (3226994944, 3226995199),
    (3226995200, 3226995455),
    (3227017984, 3227018239),
    (3227283968, 3227284223),
    (3227446016, 3227446271),
    (3227799040, 3227799295),
    (3227818496, 3227818751),
    (3228280832, 3228281087),
    (3228334080, 3228334335),
    (3229363712, 3229363967),
    (3230004224, 3230004479),
    (3231018752, 3231019007),
    (3231101952, 3231102975),
    (3231102976, 3231103231),
    (3231307008, 3231307263),
    (3231424512, 3231432703),
    (3231490560, 3231490815),
    (3231760896, 3231761151),
    (3232235520, 3232301055),
    (3232464896, 3232481279),
    (3232481280, 3232483327),
    (3232563456, 3232563711),
    (3232563712, 3232564223),
    (3232564224, 3232564479),
    (3232595968, 3232598015),
    (3232825344, 3232890879),
    (3233415168, 3233431551),
    (3233586432, 3233586687),
    (3233586688, 3233586943),
    (3233728768, 3233729023),
    (3233729024, 3233729279),
    (3234015744, 3234016255),
    (3234031872, 3234032127),
    (3234034688, 3234035199),
    (3234035200, 3234035455),
    (3237560320, 3237564415),
    (3237670912, 3237675007),
    (3240105472, 3240105727),
    (3240485120, 3240485375),
    (3240488960, 3240491007),
    (3240529664, 3240529919),
    (3240579072, 3240581119),
    (3240602624, 3240603647),
    (3240612864, 3240613119),
    (3241934848, 3242196991),
    (3245044736, 3245045759),
    (3246526208, 3246526463),
    (3246726144, 3246726655),
    (3247068672, 3247068927),
    (3256885248, 3256889343),
    (3257097472, 3257097727),
    (3257121280, 3257121535),
    (3257122816, 3257131007),
    (3257135360, 3257135615),
    (3257139456, 3257139711),
    (3257139712, 3257140223),
    (3257170176, 3257170431),
    (3257178112, 3257180159),
    (3258767872, 3258768127),
    (3259105280, 3259170815),
    (3259836658, 3259836658),
    (3259836662, 3259836662),
    (3260022784, 3260284927),
    (3262043648, 3262043903),
    (3267043328, 3267044351),
    (3284271104, 3284402175),
    (3322705920, 3322706687),
    (3323068416, 3323199487),
    (3325256704, 3325256959),
    (3331387392, 3331391487),
    (3340859392, 3340859647),
    (3340860416, 3340861439),
    (3341849344, 3341849599),
    (3343172608, 3343173631),
    (3347050496, 3347052543),
    (3350964224, 3350965247),
    (3351047680, 3351048191),
    (3355430912, 3355431167),
    (3405803776, 3405804031),
    (3406562816, 3406563071),
    (3423420416, 3423422463),
    (3423649792, 3423651839),
    (3423858176, 3423858431),
    (3427454976, 3427459071),
    (3429980928, 3429981183),
    (3439329280, 3456106495),
    (3449797888, 3449798143),
    (3450077184, 3450093567),
    (3453059072, 3453075455),
    (3463197696, 3463198207),
    (3466920960, 3466921215),
    (3494717440, 3494719487),
    (3494904832, 3494905855),
    (3497778944, 3497779199),
    (3509827840, 3509828095),
    (3509989376, 3509993471),
    (3513499648, 3513500159),
    (3513504256, 3513504511),
    (3513504768, 3513505023),
    (3550244352, 3550244863),
    (3564699648, 3564707839),
    (3564748800, 3564756991),
    (3571122176, 3571187711),
    (3590324224, 3607101439),
    (3607101440, 3623878655),
    (3629326592, 3629330943),
    (3629331200, 3629334527),
    (3633821440, 3633821695),
    (3633823232, 3633823743),
    (3635183616, 3635191807),
    (3636012032, 3636012287),
    (3638225152, 3638225407),
    (3638587392, 3638591487),
    (3758096384, 4026531839),
    (3925606400, 3925606655),
    (4026531840, 4294967294),
    (4294967294, LAST_NUMBR),
    (4294967295, 4294967295),
];
