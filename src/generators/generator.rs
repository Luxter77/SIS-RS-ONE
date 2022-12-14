use crate::{r#static::*, display::display, message::{MessageToCheck, MessageToPrintOrigin}, generators::*, resolv::check_reserved};

use std::{sync::atomic::Ordering, time::Duration, thread::park_timeout};

pub(crate) fn generate(mut worker_handles: ThreadHandler<()>, args: CommandLineArguments, zip: u32) -> IPGenerator {
    // let skip: u128, seed: u128, last: u128, zip: u32, use_zip: bool, no_continue: bool, strategy: NumberGenerators
    let mut generator: IPGenerator = IPGenerator::new(args.seed, args.generator_strategy, args.no_continue, args.last);
    let mut skip:      u128        = args.skip;

    if skip > u32::MAX.into() {
        generator.gen_skip(u32::MAX);
        skip -= u32::MAX as u128;
    };
    
    if skip != 0 { generator.gen_skip(skip.try_into().unwrap()) };
    
    if args.use_zip { generator.gen_zip(zip).unwrap(); };

    while !READY___SET_GO_SIGNAL.load(Ordering::Relaxed) { park_timeout(Duration::from_secs(2)); };

    while !GENERATOR_STOP_SIGNAL.load(Ordering::Relaxed) { // Generates IIPs for the query worker threads
        if QUEUE_TO_CHECK.size() < QUEUE_LIMIT {
            if let GeneratorMessage::Normal(co, nu) = generator.gen_next() {
                match check_reserved(nu, generator.gen_dir()) {
                    crate::resolv::ReservedResoult::Valid        => { QUEUE_TO_CHECK.add(MessageToCheck::ToCheck(co, nu)); },
                    crate::resolv::ReservedResoult::Skip(n) => { 
                        match generator {
                            IPGenerator::Sequential(_) => { generator.gen_skip(n.saturating_sub(1)) },
                            IPGenerator::PoorMans(_) => {/* noop */},
                            IPGenerator::LCG(_)      => {/* noop */},
                        };
                    },
                    crate::resolv::ReservedResoult::Invalid  => {/* noop */},
                };
            } else {
                display(MessageToPrintOrigin::Generator, "[ We went all the way arround!!!1!!11!1one!!1!111 ]"); break;
            };

            if generator.las_passed() {
                display(MessageToPrintOrigin::Generator, "[ We reached the stipulated end! ]"); break;
            };

            if generator.gen_state().0 % 15000 == 0 {
                generator.write_to_save_file().unwrap();
                if cfg!(debug_assertions) { display(MessageToPrintOrigin::Generator, &format!("[ to_check queue size is currently: {} items long; c <==> {} ]", QUEUE_TO_CHECK.size(), generator.gen_state().0)); };
            };
        } else {
            worker_handles.unpark();
            park_timeout(Duration::from_secs(SLEEP_TIME / 2));
        };
    };
    
    if cfg!(debug_assertions) {
        display(MessageToPrintOrigin::Generator, &format!("[ Final generator state {} ]", match generator.clone() {
            IPGenerator::PoorMans(gen) => format!("{:?}", gen),
            IPGenerator::Sequential(gen) => format!("{:?}", gen),
            IPGenerator::LCG(gen)      => format!("{:?}", gen), // SHOULDN'T ever panic, in theory...
        }));
    };

    QUEUE_TO_CHECK.add( MessageToCheck::End );

    display(MessageToPrintOrigin::Generator,  &match generator.write_to_save_file() {
        Err(why) => { format!("[ Coult not write to checkpoint file! {why} ][ GeneratorState: {} ]", serde_json::to_string(&generator).unwrap()) },
        Ok(_)           => { format!("[ Wrote generator state to checkpoint file {CHECKPOINT_FILE} ]") },
    });

    return generator;
}
