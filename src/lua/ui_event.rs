#[derive(Clone)]
pub enum UiEvent {
    StepLeft,
    StepRight,
    StepToStart,
    StepToEnd,
    StepWordLeft,
    StepWordRight,
    Remove,
    DeleteToEnd,
    DeleteFromStart,
    PreviousCommand,
    NextCommand,
    ScrollUp,
    ScrollDown,
    ScrollBottom,
    Complete,
}
