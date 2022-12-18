use num_bigint::BigUint;

#[derive(Clone, Debug)]
pub enum MessageToCheck {
    EmptyQueue,
    ToCheck(u128, BigUint),
    End,
}

#[derive(Clone, Debug)]
pub enum MessageToWrite {
    EmptyQueue,
    ToWrite(String, String),
    End,
}
