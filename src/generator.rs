use crate::{r#static::*, display::display, message::{MessageToCheck, MessageToPrintOrigin}, generators::*};

use std::{sync::atomic::Ordering, time::Duration, thread::sleep};

pub(crate) fn generate(skip: u128, num: u128, last: u128, zip: u32, zip_flag: bool) -> IPGenerator {
    let mut generator: IPGenerator;
    
    if let std::io::Result::Ok(gen) = IPGenerator::get_from_save_file() {
        generator = gen;
    } else {   
        generator = IPGenerator::new(num);
    };

    if skip != 0 { generator.gen_skip(skip); };
    if zip_flag  { generator.gen_zip(zip).unwrap(); };

    loop { // Generates IIPs for the query worker threads
        if GENERATOR_STOP_SIGNAL.load(Ordering::Relaxed) { break };
        if QUEUE_TO_CHECK.size() < QUEUE_LIMIT * 10 {
            if let GeneratorMessage::Normal(co, nu) = generator.gen_next() {
                QUEUE_TO_CHECK.add( MessageToCheck::ToCheck(co, nu) );
            } else {
                display(MessageToPrintOrigin::GeneratorThread, "[ We went all the way arround!!!1!!11!1one!!1!111 ]"); break;
            };

            if cfg!(any(feature = "PRand-LCG", feature = "Sequential-Generator")) && last == generator.get_las() {
                display(MessageToPrintOrigin::GeneratorThread, "[ We reached the stipulated end! ]"); break;
            };

            if cfg!(debug_assertions) { display(MessageToPrintOrigin::GeneratorThread, &format!("[ to_check queue size is currently: {} items long; c <==> {} ]", QUEUE_TO_CHECK.size(), generator.gen_state().0)); };
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

    display(MessageToPrintOrigin::GeneratorThread,  &match generator.write_to_save_file() {
        Err(why) => { format!("[ Coult not write to checkpoint file! {why} ][ GeneratorState: {} ]", serde_json::to_string(&generator).unwrap()) },
        Ok(_)           => { format!("[ Wrote generator state to checkpoint file {CHECKPOINT_FILE} ]") },
    });

    return generator;
}
