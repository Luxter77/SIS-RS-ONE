use crate::{r#static::*, display::display, message::{MessageToCheck, MessageToPrintOrigin}, generators::*, resolv::check_reserved};

use std::{sync::atomic::Ordering, time::Duration, thread::park_timeout};

pub(crate) fn generate(skip: u128, seed: u128, last: u128, zip: u32, use_zip: bool, no_continue: bool, strategy: NumberGenerators, mut worker_handles: ThreadHandler<()>) -> IPGenerator {
    let mut generator: IPGenerator = IPGenerator::new(seed, strategy, no_continue);

    if skip != 0 { generator.gen_skip(skip); };
    if use_zip   { generator.gen_zip(zip).unwrap(); };

    while !READY___SET_GO_SIGNAL.load(Ordering::Relaxed) {};

    while !GENERATOR_STOP_SIGNAL.load(Ordering::Relaxed) { // Generates IIPs for the query worker threads
        if QUEUE_TO_CHECK.size() < QUEUE_LIMIT * 10 {
            if let GeneratorMessage::Normal(co, nu) = generator.gen_next() {
                match check_reserved(nu) {
                    crate::resolv::ReservedResoult::Valid        => { QUEUE_TO_CHECK.add(MessageToCheck::ToCheck(co, nu)); },
                    crate::resolv::ReservedResoult::Skip(n) => { 
                        match generator {
                            IPGenerator::SequentialGenerator(_) => { generator.gen_skip(n.saturating_sub(1) as u128) },
                            _ => {}, // lol
                        };
                    },
                    crate::resolv::ReservedResoult::Overflow     => todo!(),
                };
            } else {
                display(MessageToPrintOrigin::GeneratorThread, "[ We went all the way arround!!!1!!11!1one!!1!111 ]"); break;
            };

            if generator.can_last() && (last == generator.get_las()) {
                display(MessageToPrintOrigin::GeneratorThread, "[ We reached the stipulated end! ]"); break;
            };

            if cfg!(debug_assertions) && (generator.gen_state().0 % 100 == 0) {
                display(MessageToPrintOrigin::GeneratorThread, &format!("[ to_check queue size is currently: {} items long; c <==> {} ]", QUEUE_TO_CHECK.size(), generator.gen_state().0));
            };
        } else {
            worker_handles.unpark();
            park_timeout(Duration::from_secs(SLEEP_TIME / 2));
        };
    };
    
    #[cfg(debug_assertions)]
    display(MessageToPrintOrigin::GeneratorThread, &format!("[ Final generator state {} ]", match generator.clone() {
        IPGenerator::PoorMansIPGenerator(gen) => format!("{:?}", gen),
        IPGenerator::SequentialGenerator(gen) => format!("{:?}", gen),
        IPGenerator::LCGIPGenerator(gen) => format!("{:?}", gen), // SHOULDN'T ever panic, in theory...
    }));

    QUEUE_TO_CHECK.add( MessageToCheck::End );

    display(MessageToPrintOrigin::GeneratorThread,  &match generator.write_to_save_file() {
        Err(why) => { format!("[ Coult not write to checkpoint file! {why} ][ GeneratorState: {} ]", serde_json::to_string(&generator).unwrap()) },
        Ok(_)           => { format!("[ Wrote generator state to checkpoint file {CHECKPOINT_FILE} ]") },
    });

    return generator;
}
