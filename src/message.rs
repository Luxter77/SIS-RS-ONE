use num_bigint::BigUint;

#[derive(Clone, Debug)]
pub enum MessageToCheck {
    EmptyQueue,
    ToCheck(BigUint, BigUint),
    End,
}

#[derive(Clone, Debug)]
pub enum MessageToWrite {
    EmptyQueue,
    ToWrite(String, String),
    End,
}

#[derive(Clone, Debug)]
pub enum MessageToPrintOrigin {
    GeneratorThread,
    QueryerThread,
    WriterThread,
    DisplayThread,
    MainThread,
}

#[derive(Clone, Debug)]
pub enum MessageToPrint {
    EmptyQueue,
    ToDisplay(MessageToPrintOrigin, String),
    End,
}
