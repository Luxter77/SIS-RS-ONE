#![allow(non_snake_case)]

use dns_lookup::lookup_addr;

// use pad::{Alignment, PadStr};

use trust_dns_resolver::{ Resolver, config::{NameServerConfig, Protocol, ResolverConfig, ResolverOpts} };

use std::net::SocketAddr;
use std::{
    net::{IpAddr, Ipv4Addr},
    sync::atomic::Ordering,
};

use crate::{
    generators::{IPGenerator, GeneratorDirection},
    display::display,
    r#static::*,
    message::*,
};

pub enum ReservedResoult {
    Valid,
    Invalid,
    Skip(u32),
}

pub fn check_reserved(num: u32, dir: GeneratorDirection) -> ReservedResoult {
    for (start, end) in NO_GO_RANGES {
        if ((start) <= num) && (num <= (end)) {
            return match dir {
                GeneratorDirection::Forward =>  ReservedResoult::Skip(end - num),
                GeneratorDirection::Backward => ReservedResoult::Skip(num - start),
                GeneratorDirection::Random =>   ReservedResoult::Invalid,
            };
        };
    };

    return ReservedResoult::Valid;
}

/// Counts how many posible distinct numbers can this program (using current filters) generate
pub fn count_posibilites(clamp: u128) -> u128 {
    let mut count: u128 = NEXT_PRIME;
    for (s, e) in NO_GO_RANGES { count -= (e - s) as u128; };
    return count.clamp(0u128, clamp);
}

fn trust_dns_lookup_addr(lipn: &mut Vec<String>, ip: &Ipv4Addr, resolver: &Resolver, t_use_host_resolver: bool) {
    if let Ok(res) = resolver.reverse_lookup(IpAddr::V4(ip.to_owned())) {
        let ips: Vec<String> = res.iter().map( |nam| -> String { nam.to_ascii() } ).collect();
        
        if ips.len() > 1 && cfg!(debug_assertions) {
            display(MessageToPrintOrigin::Queryer, &format!("[ IP HAS MORE THAN ONE ADRESS! -> {:?} ]", ips));
        };
        
        if t_use_host_resolver && !lipn.is_empty() {
            let mut h_res_conf: ResolverConfig = ResolverConfig::new();          
            h_res_conf.add_name_server(NameServerConfig::new(SocketAddr::new(IpAddr::V4(*ip), 53), Protocol::default()));
            if let Ok(h_res) = Resolver::new(h_res_conf, ResolverOpts::default()).unwrap().reverse_lookup(IpAddr::V4(ip.to_owned())) {
                lipn.extend(h_res.iter().map( |nam| -> String { nam.to_ascii() } ).collect::<std::collections::HashSet<_>>());
            };
        };
        
        lipn.extend(ips.iter().map( move | nam: &String | nam.to_owned() ).collect::<std::collections::HashSet<_>>());
    };
}

pub(crate) fn resolv_worker(t_use_host_resolver: bool, t_use_trust_dns: bool, t_use_system_dns: bool, mut generator_handle: ThreadHandler<IPGenerator>) {
    let mut resolver: Option<trust_dns_resolver::Resolver> = Option::None;

    let mut pending: bool = false;
    let mut found:   bool = false;
    
    // logic too deepth for the compiler?
    // This will never get read, but the all knowing compiler insists...
    let mut max_pos: u128 = u32::MAX.into();
    let mut iip:     u32  = 0u32;
    let mut c:       u128 = 0u128;
    
    
    let mut p:       f32;

    if cfg!(feature = "PRand-LCG") { max_pos = LAST_NUMBR; }
    if t_use_trust_dns {
        resolver = Some(trust_dns_resolver::Resolver::default().unwrap());
    }

    while !READY___SET_GO_SIGNAL.load(Ordering::Relaxed) { std::thread::park_timeout(std::time::Duration::from_millis(2)); };

    while !QUERYER___STOP_SIGNAL.load(Ordering::Relaxed) {        
        if let Ok( MessageToCheck::End ) = QUEUE_TO_CHECK.peek() { break };
        
        if let Ok( MessageToCheck::ToCheck(p_c, p_iip) ) = QUEUE_TO_CHECK.get() {
            (c, iip, pending) = (p_c, p_iip, true);
        } else {
            generator_handle.unpark();
        };

        if pending {
            p = c as f32 * 100.0f32 / max_pos as f32;
            
            let mut lipn:   Vec<String> = Vec::new();
            
            let     ip:     Ipv4Addr    = Ipv4Addr::from(iip);
            
            if t_use_system_dns {
                lipn.push(lookup_addr(&ip.into()).unwrap());
            } else if let Some(ref resolver) = resolver {
                trust_dns_lookup_addr(&mut lipn, &ip, resolver, t_use_host_resolver);
            };
            
            lipn.sort_by(| a, b | human_sort::compare(a.as_str(), b.as_str()));
            
            for ipn in lipn {
                found = true;
                let [x, y, z, w] = ip.clone().octets();
                if ipn != ip.to_string() {
                    FOUND_COUNT.add_one();
                    display(MessageToPrintOrigin::Queryer, &format!("[ {p:0>17}% ][ {c:>10} / {max_pos} ][ IP: {x:<3}.{y:<3}.{z:<3}.{w:<3} ][ DNS: {ipn} ]"));
                    QUEUE_TO_WRITE.add(MessageToWrite::ToWrite(ip.to_string(), ipn) );
                } else if cfg!(debug_assertions) {
                    display(MessageToPrintOrigin::Queryer, &format!("[ {p:0>17}% ][ {c:>10} / {max_pos} ][ IP: {x:<3}.{y:<3}.{z:<3}.{w:<3} ][ IPN: {ipn} ]"));
                };
            };

            if found {
                FOUND_DISTINCT_COUNT.add_one();
                found = false;
            };
            
            pending = false;
        } else {
            std::thread::park_timeout(std::time::Duration::from_millis(SLEEP_TIME * 3));
        };

        // if cfg!(debug_assertions) { display(MessageToPrintOrigin::QueryerThread, &format!("[ to_write queue size is currently: {} items long. ]", QUEUE_TO_CHECK.size())); };
    };
}
