use crate::{r#static::*, display::display, message::{MessageToCheck, MessageToPrintOrigin}};

use std::{sync::atomic::Ordering, time::Duration, thread::sleep};

#[allow(dead_code)]
/// Counts how many posible distinct numbers can this program (using current filters) generate
pub fn count_posibilites(clamp: u128) -> u128 {
    let mut count: u128 = NEXT_PRIME;
    for (s, e) in NO_GO_RANGES { count -= e - s; };
    return count.clamp(0u128, clamp);
}

pub(crate) fn generate(mut skip: u128, mut num: u128, last: u128, zip: u128, mut zip_flag: bool) -> (u128, u128) {
    let mut c: u128 = 0u128;
    
    let first_number: u128 = num.clone();

    let mut send: bool;

    loop { // Generates IIPs for the query worker threads
        if GENERATOR_STOP_SIGNAL.load(Ordering::Relaxed) { break };

        let can_go: bool = QUEUE_TO_CHECK.size() < QUEUE_LIMIT * 10;

        if can_go {
            c += 1u128;
            
            if skip == 0u128 {
                send = true;
            } else {
                skip -= 1u128;
                send = false;
            };

            if zip_flag {
                if num != zip {
                    send = false;
                } else {
                    zip_flag = false;
                }
            } 

            if send { QUEUE_TO_CHECK.add( MessageToCheck::ToCheck(c.clone(), num.clone()) ); };

            num = (((A_PRIMA % M_PRIMA) * (num % M_PRIMA)) % M_PRIMA) + (C_PRIMA % M_PRIMA);
            
            if num == first_number {
                display(MessageToPrintOrigin::GeneratorThread, "[ We went all the way arround!!!1!!11!1one!!1!111 ]"); break;
            };
            
            if c == last {
                display(MessageToPrintOrigin::GeneratorThread, "[ We reached the stipulated end! ]"); break;
            }
            
            if cfg!(debug_assertions) { display(MessageToPrintOrigin::GeneratorThread, &format!("[ to_check queue size is currently: {} items long; c <==> {} ]", QUEUE_TO_CHECK.size(), c.clone())); };
        } else {
            sleep(Duration::from_secs(SLEEP_TIME / 2));
        };
    };

    QUEUE_TO_CHECK.add( MessageToCheck::End );

    return (num, c);
}
