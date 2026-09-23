use super::{Actor, SessionToken, SignedIn};
use crate::models::sql_types::Role;

/// The whole point of the type. A token that reaches a log line is a
/// token in every copy of that log, and this is the check that a
/// `{:?}` somebody adds in a hurry cannot put one there, through the
/// struct it actually travels in as well as on its own.
#[test]
fn debug_output_keeps_the_secret() {
    let secret = "0123456789abcdef";
    let t = SessionToken::new(secret.to_owned());
    assert_eq!(format!("{t:?}"), "SessionToken(..)");
    assert_eq!(format!("{:?}", Some(t.clone())), "Some(SessionToken(..))");

    let signed_in = SignedIn {
        token: t,
        actor: Actor {
            user_id: 1,
            shop_id: 1,
            role: Role::Owner,
            session_id: 1,
        },
        name: "Propriétaire".to_owned(),
    };
    let printed = format!("{signed_in:?}");
    assert!(!printed.contains(secret), "{printed}");
}
