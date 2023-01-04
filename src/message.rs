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

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub enum MessageToPrintOrigin {
    Custom(String),
    Generator,
    Queryer,
    Writer,
    Display,
    Main,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MessageToPrint {
    EmptyQueue,
    ToDisplay(MessageToPrintOrigin, String),
    Wait(std::time::Duration),
    End,
}
