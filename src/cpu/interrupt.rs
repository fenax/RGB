pub enum Interrupt{
    None,
    VBlank,
    LcdcStatus,
    TimerOverflow,
    SerialTransfer,
    Button,
    DoDmaTransfer
}