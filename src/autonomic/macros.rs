/// Macro to quickly generate a list of AutonomicActions.
/// Usage:
/// ```
/// use dteam::autonomic_actions;
/// let actions = autonomic_actions![
///     (1, Recommend, Low, "Optimize flow"),
///     (2, Notify, Low, "Alert admin")
/// ];
/// ```
#[macro_export]
macro_rules! autonomic_actions {
    ($( ($id:expr, $type:ident, $risk:ident, $params:expr) ),*) => {
        {
            use $crate::autonomic::types::{AutonomicAction, ActionType, ActionRisk};
            vec![
                $(
                    AutonomicAction::new($id, ActionType::$type, ActionRisk::$risk, $params)
                ),*
            ]
        }
    };
}
