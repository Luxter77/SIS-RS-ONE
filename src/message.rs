#[derive(Clone, Debug, PartialEq)]
pub enum MessageToCheck {
    EmptyQueue,
    ToCheck(u128, u32),
    End,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MessageToWrite {
    EmptyQueue,
    ToWrite(String, String),
    End,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MessageToPrintOrigin {
    GeneratorThread,
    QueryerThread,
    WriterThread,
    DisplayThread,
    MainThread,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MessageToPrint {
    EmptyQueue,
    ToDisplay(MessageToPrintOrigin, String),
    Wait(std::time::Duration),
    End,
}
