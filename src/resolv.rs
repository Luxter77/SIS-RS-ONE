#![allow(non_snake_case)]

use dns_lookup::lookup_addr;
use num_bigint::BigUint;
#[cfg(debug_assertions)]
use pad::{Alignment, PadStr};

use queues::{IsQueue, Queue};
#[cfg(feature = "trust-dns")]
use trust_dns_resolver::Resolver;

#[cfg(feature = "host-resolv")]
use trust_dns_resolver::config::{NameServerConfig, Protocol, ResolverConfig, ResolverOpts};

use human_sort;

#[cfg(feature = "host-resolv")]
use std::net::SocketAddr;
use std::{
    net::{IpAddr, Ipv4Addr},
    sync::{atomic::Ordering, Arc, Mutex},
};

use crate::message::*;
use crate::r#static::*;

fn check_reserved(num: BigUint) -> bool {
    if num > BigUint::from(MAX_IIP) {
        return false;
    };

    for (start, end) in NO_GO_RANGES {
        if (BigUint::from(start) <= num) && (num <= BigUint::from(end)) {
            return false;
        };
    };

    return true;
}

#[cfg(feature = "trust-dns")]
fn trust_dns_lookup_addr(lipn: &mut Vec<String>, ip: &Ipv4Addr, resolver: &Resolver) {
    if let Ok(res) = resolver.reverse_lookup(IpAddr::V4(ip.to_owned())) {
        #[cfg(debug_assertions)] {
            let ips: Vec<String> = res.iter().map( |nam| -> String { nam.to_ascii() } ).collect();
            if ips.len() > 1 { println!("{}", format!("IP HAS MORE THAN ONE ADRESS! -> {:?}", ips)); };
            lipn.extend(ips.iter().map( move | nam: &String | nam.to_owned() ).collect::<std::collections::HashSet<_>>());
        };

        #[cfg(not(debug_assertions))] lipn.extend(res.iter().map( |nam| -> String { nam.to_ascii() } ).collect::<std::collections::HashSet<_>>());
        
        #[cfg(feature = "host-resolv")]  {
            if lipn.len() > 0 {
                let mut h_res_conf = ResolverConfig::new();          
                h_res_conf.add_name_server(NameServerConfig::new(SocketAddr::new(IpAddr::V4(ip.clone()), 53), Protocol::default()));
                if let Ok(h_res) = Resolver::new(h_res_conf, ResolverOpts::default()).unwrap().reverse_lookup(IpAddr::V4(ip.to_owned())) {
                    lipn.extend(h_res.iter().map( |nam| -> String { nam.to_ascii() } ).collect::<std::collections::HashSet<_>>());
                };
            };
        };
    };
}

pub(crate) fn resolv_worker(queue: Arc<Mutex<Queue<MessageToCheck>>>, out_queue: Arc<Mutex<Queue<MessageToWrite>>>) {
    let mut pending: bool = false;

    // logic too deepth for the compiler?
    // This will never get read, but the all knowing compiler insists...
    let mut iip:     BigUint = BigUint::from(0u128); 
    let mut c:       u128    =               0u128;

    let mut p:       f32;

    #[cfg(feature = "trust-dns")] let resolver: trust_dns_resolver::Resolver = trust_dns_resolver::Resolver::default().unwrap();

    loop {
        if QUERYER___STOP_SIGNAL.load(Ordering::Relaxed) { break };
        
        if let Ok( MessageToCheck::End ) = queue.lock().unwrap().peek() { break };
        
        if let Ok( MessageToCheck::ToCheck(p_c, p_iip) ) = queue.lock().unwrap().remove() {
            (c, iip, pending) = (p_c.clone(), p_iip.clone(), true);
        };

        if pending {
            p  = (c as f32) * 100.0f32 / (LAST_NUMBR as f32);

            if check_reserved(iip.clone()) {
                let mut lipn:   Vec<String> = Vec::new();
                
                let     ip:     Ipv4Addr    = Ipv4Addr::from(iip.to_string().parse::<u32>().unwrap());

                if crate::r#static::USE_SYSTEM_DNS {
                    lipn.push(lookup_addr(&ip.into()).unwrap());
                } else {
                    #[cfg(feature = "trust-dns")]
                    trust_dns_lookup_addr(&mut lipn, &ip, &resolver);
                };
                
                lipn.sort_by(| a, b | human_sort::compare(a.as_str(), b.as_str()));
                
                for ipn in lipn {
                    let [x, y, z, w] = ip.clone().octets();
                    if ipn != ip.to_string() {
                        println!("{}", format!("[ {p:>17}% ][ {a:>10} / {t} ][ IP: {x:<3}.{y:<3}.{z:<3}.{w:<3} ][ DNS: {d} ]", a=c, p=p, t=LAST_NUMBR, x=x, y=y, z=z, w=w, d=ipn));
                        out_queue.lock().unwrap().add(MessageToWrite::ToWrite(ip.to_string(), ipn) ).unwrap();
                    } else {
                        #[cfg(debug_assertions)] println!("{}", format!("[ {p:>17}% ][ {a:>10} / {t} ][IP: {x:<3}.{y:<3}.{z:<3}.{w:<3} ][ IPN: {d} ]", a=c, p=p, t=LAST_NUMBR, x=x, y=y, z=z, w=w, d=ipn));
                    };
                };
            } else {
                #[cfg(debug_assertions)] println!("{}", format!("[ {p:>17}% ][ {a:>10} / {t} ][ IP: {b:>15} ][ MSG: REJECTED! ]", a=c, p=p, t=LAST_NUMBR, b=iip.clone().to_string().pad_to_width_with_alignment(15, Alignment::Right)));
            };
            std::io::Write::flush(&mut std::io::stdout()).expect("\n\rUnable to flush stdout!");
            pending = false;
        } else {
            std::thread::sleep(std::time::Duration::from_millis(SLEEP_TIME * 3));
        };

        // #[cfg(debug_assertions)] println!("{}", format!("to_write queue size is currently: {} items long.", queue.lock().unwrap().size()));
    };
}
