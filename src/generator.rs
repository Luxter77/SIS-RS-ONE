use crate::{r#static::*, display::display};

use std::{sync::atomic::Ordering, time::Duration, thread::sleep};
use num_bigint::BigUint;
use crate::message::{MessageToCheck, MessageToPrintOrigin};

#[allow(dead_code)]
/// Counts how many posible distinct numbers can this program (using current filters) generate
pub fn count_posibilites(clamp: BigUint) -> BigUint {
    let mut count: BigUint = BigUint::from(NEXT_PRIME);
    for (s, e) in NO_GO_RANGES { count -= e - s; };
    return count.clamp(ZERO.clone(), clamp);
}

pub(crate) fn generate(mut skip: BigUint, mut num: BigUint, last: BigUint, zip: BigUint, mut zip_flag: bool) -> (BigUint, BigUint) {
    let mut c: BigUint = ZERO.clone();
    
    let first_number: BigUint = num.clone();

    let mut send: bool;

    loop { // Generates IIPs for the query worker threads
        if GENERATOR_STOP_SIGNAL.load(Ordering::Relaxed) { break };

        let can_go: bool = QUEUE_TO_CHECK.size() < QUEUE_LIMIT * 10;

        if can_go {
            c += ONE.clone();
            
            if skip == ZERO.clone() {
                send = true;
            } else {
                skip -= ONE.clone();
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

            num = (BigUint::from(A_PRIMA) * num + BigUint::from(C_PRIMA)) % BigUint::from(M_PRIMA);
            
            if num == first_number {
                display(MessageToPrintOrigin::GeneratorThread, "[ We went all the way arround!!!1!!11!1one!!1!111 ]"); break;
            };
            
            if num == last {
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
