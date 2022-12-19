#![allow(non_snake_case)]

use num_traits::cast::ToPrimitive;
use dns_lookup::lookup_addr;
use num_bigint::BigUint;

use pad::{Alignment, PadStr};

#[cfg(feature = "trust-dns")]
use trust_dns_resolver::{ Resolver, config::{NameServerConfig, Protocol, ResolverConfig, ResolverOpts} };

use human_sort;
use std::net::SocketAddr;
use std::{
    net::{IpAddr, Ipv4Addr},
    sync::atomic::Ordering,
};

use crate::message::*;
use crate::r#static::*;
use crate::display::display;

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
        let ips: Vec<String> = res.iter().map( |nam| -> String { nam.to_ascii() } ).collect();
        
        if ips.len() > 1 && cfg!(debug_assertions) {
            display(MessageToPrintOrigin::QueryerThread, &format!("[ IP HAS MORE THAN ONE ADRESS! -> {:?} ]", ips));
        };
        
        if cfg!(feature = "host-resolv") {
            if lipn.len() > 0 {
                let mut h_res_conf = ResolverConfig::new();          
                h_res_conf.add_name_server(NameServerConfig::new(SocketAddr::new(IpAddr::V4(ip.clone()), 53), Protocol::default()));
                if let Ok(h_res) = Resolver::new(h_res_conf, ResolverOpts::default()).unwrap().reverse_lookup(IpAddr::V4(ip.to_owned())) {
                    lipn.extend(h_res.iter().map( |nam| -> String { nam.to_ascii() } ).collect::<std::collections::HashSet<_>>());
                };
            };
        };
        
        lipn.extend(ips.iter().map( move | nam: &String | nam.to_owned() ).collect::<std::collections::HashSet<_>>());
    };
}

pub(crate) fn resolv_worker() {
    let mut pending: bool = false;
    let mut found:   bool = false;

    // logic too deepth for the compiler?
    // This will never get read, but the all knowing compiler insists...
    let mut iip:     BigUint = BigUint::from(0u128);
    let mut c:       BigUint = BigUint::from(0u128);

    let mut p:       f32;

    #[cfg(feature = "trust-dns")] let resolver: trust_dns_resolver::Resolver = trust_dns_resolver::Resolver::default().unwrap();

    loop {
        if QUERYER___STOP_SIGNAL.load(Ordering::Relaxed) { break };
        
        if let Ok( MessageToCheck::End ) = QUEUE_TO_CHECK.peek() { break };
        
        if let Ok( MessageToCheck::ToCheck(p_c, p_iip) ) = QUEUE_TO_CHECK.get() {
            (c, iip, pending) = (p_c.clone(), p_iip.clone(), true);
        };

        if pending {
            p  = (c.clone() * BigUint::from(100u128) / LAST_NUMBR).to_f32().expect("AUCHI");

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
                    found = true;
                    let [x, y, z, w] = ip.clone().octets();
                    if ipn != ip.to_string() {
                        F_COUNT.add_one();
                        display(MessageToPrintOrigin::QueryerThread, &format!("[ {p:>17}% ][ {a:>10} / {t} ][ IP: {x:<3}.{y:<3}.{z:<3}.{w:<3} ][ DNS: {d} ]", a=c, p=p, t=LAST_NUMBR, x=x, y=y, z=z, w=w, d=ipn));
                        QUEUE_TO_WRITE.add(MessageToWrite::ToWrite(ip.to_string(), ipn) );
                    } else if cfg!(debug_assertions) {
                        display(MessageToPrintOrigin::QueryerThread, &format!("[ {p:>17}% ][ {a:>10} / {t} ][IP: {x:<3}.{y:<3}.{z:<3}.{w:<3} ][ IPN: {d} ]", a=c, p=p, t=LAST_NUMBR, x=x, y=y, z=z, w=w, d=ipn));
                    };
                };
            } else {
                if cfg!(debug_assertions) { display(MessageToPrintOrigin::QueryerThread, &format!("[ {p:>17}% ][ {a:>10} / {t} ][ IP: {b:>15} ][ MSG: REJECTED! ]", a=c, p=p, t=LAST_NUMBR, b=iip.clone().to_string().pad_to_width_with_alignment(15, Alignment::Right))); };
            };

            if found {
                F_D_COUNT.add_one();
                found = false;
            };
            
            pending = false;
        } else {
            std::thread::sleep(std::time::Duration::from_millis(SLEEP_TIME * 3));
        };

        // if cfg!(debug_assertions) { display(MessageToPrintOrigin::QueryerThread, &format!("[ to_write queue size is currently: {} items long. ]", QUEUE_TO_CHECK.size())); };
    };
}
