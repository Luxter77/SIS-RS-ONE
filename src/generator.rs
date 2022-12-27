use crate::{r#static::*, display::display, message::{MessageToCheck, MessageToPrintOrigin}, generators::*};

use std::{sync::atomic::Ordering, time::Duration, thread::sleep};

pub(crate) fn generate(skip: u128, seed: u128, last: u128, zip: u32, use_zip: bool, strategy: NumberGenerators) -> (u128, u32) {
    let (use_las, mut generator): (bool, IPGenerator) = match strategy {
        NumberGenerators::PoorMansGen => (false, IPGenerator::PoorMansIPGenerator(PoorMansIPGenerator::default())),
        NumberGenerators::Sequential  => (true,  IPGenerator::SequentialGenerator(SequentialGenerator::default())),
        NumberGenerators::LCG => {
            display(MessageToPrintOrigin::GeneratorThread, &format!("[ WARNING: THE IMPLEMENTATION OF THE FEATURE PRand-LCG IS CURRENTLY BROKEN! ]"));
            QUEUE_TO_PRINT.add( crate::message::MessageToPrint::Wait(std::time::Duration::from_secs(3)) );
            (true, IPGenerator::LCGIPGenerator(LCGIPGenerator::new(seed, M_PRIMA, C_PRIMA, A_PRIMA)))
        },
    };

    if skip != 0 { generator.gen_skip(skip); };
    if use_zip   { generator.gen_zip(zip).unwrap(); };

    while !GENERATOR_STOP_SIGNAL.load(Ordering::Relaxed) { // Generates IIPs for the query worker threads
        if QUEUE_TO_CHECK.size() < QUEUE_LIMIT * 10 {
            if let GeneratorMessage::Normal(co, nu) = generator.gen_next() {
                QUEUE_TO_CHECK.add( MessageToCheck::ToCheck(co, nu) );
            } else {
                display(MessageToPrintOrigin::GeneratorThread, "[ We went all the way arround!!!1!!11!1one!!1!111 ]"); break;
            };

            if use_las && last == generator.get_las() {
                display(MessageToPrintOrigin::GeneratorThread, "[ We reached the stipulated end! ]"); break;
            };

            if cfg!(debug_assertions) && (QUEUE_TO_CHECK.size() % 100 == 0) { display(MessageToPrintOrigin::GeneratorThread, &format!("[ to_check queue size is currently: {} items long; c <==> {} ]", QUEUE_TO_CHECK.size(), generator.gen_state().0)); };
        } else {
            sleep(Duration::from_secs(SLEEP_TIME / 2));
        };
    };
    
    #[cfg(debug_assertions)]
    display(MessageToPrintOrigin::GeneratorThread, &format!("[ Final generator state {} ]", match generator.clone() {
        IPGenerator::PoorMansIPGenerator(gen) => format!("{:?}", gen),
        IPGenerator::SequentialGenerator(gen) => format!("{:?}", gen),
        IPGenerator::LCGIPGenerator(gen) => format!("{:?}", gen), // SHOULDN'T ever panic, in theory...
    }));

    QUEUE_TO_CHECK.add( MessageToCheck::End );

    return generator.gen_state();
}
