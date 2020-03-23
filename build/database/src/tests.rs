use super::*;
use std::error::Error;

use bibifi_util::hash;

#[test]
// test all basic stuff
fn basic_full_1() -> Result<(), Box<dyn Error>> {
    let mut my_database = Database::new(hash("wolla".to_string()));

    //admin with correct password checks true
    assert_eq!(
        my_database.check_pass(&"admin".to_string(), &hash("wolla".to_string())),
        true
    );

    //admin with wrong password checks false
    assert_eq!(
        my_database.check_pass(&"admin".to_string(), &hash("wollabig".to_string())),
        false
    );

    //anyone principal rejected with any password
    assert_eq!(
        my_database.check_pass(&"anyone".to_string(), &hash("".to_string()))
            | my_database.check_pass(&"anyone".to_string(), &hash("wolla".to_string())),
        false
    );

    //check non admin principal
    assert_eq!(
        my_database.check_pass(&"not_admin".to_string(), &hash("wolla".to_string())),
        false
    );

    //add principals to database
    my_database.create_principal(&"bob".to_string(), &hash("".to_string())); // empty string password
    my_database.create_principal(&"tom".to_string(), &hash("tom_pass".to_string()));

    //admin with correct password checks true
    assert_eq!(
        my_database.check_pass(&"bob".to_string(), &hash("".to_string()))
            && my_database.check_pass(&"tom".to_string(), &hash("tom_pass".to_string())),
        true
    );

    //admin with wrong password checks false
    assert_eq!(
        my_database.check_pass(&"bob".to_string(), &hash("wolla".to_string()))
            | my_database.check_pass(&"tom".to_string(), &hash("".to_string()))
            | my_database.check_pass(&"tom".to_string(), &hash("tom".to_string())),
        false
    );

    //check existing principal
    assert_eq!(
        my_database.check_principal(&"admin".to_string())
            && my_database.check_principal(&"anyone".to_string())
            && my_database.check_principal(&"bob".to_string())
            && my_database.check_principal(&"tom".to_string()),
        true
    );

    //check non-existing principal
    assert_eq!(
        my_database.check_principal(&"".to_string())
            | my_database.check_principal(&"none".to_string())
            | my_database.check_principal(&"tick".to_string()),
        false
    );

    // lets say, bob created my_var and delegated all permissions to everyone
    my_database.delegate(
        &Target::Variable("my_var1".to_string()),
        &"admin".to_string(),
        &Right::Read,
        &"bob".to_string(),
    );
    my_database.delegate(
        &Target::Variable("my_var1".to_string()),
        &"admin".to_string(),
        &Right::Write,
        &"bob".to_string(),
    );
    my_database.delegate(
        &Target::Variable("my_var1".to_string()),
        &"admin".to_string(),
        &Right::Append,
        &"bob".to_string(),
    );
    my_database.delegate(
        &Target::Variable("my_var1".to_string()),
        &"admin".to_string(),
        &Right::Delegate,
        &"bob".to_string(),
    );

    my_database.delegate(
        &Target::Variable("my_var1".to_string()),
        &"bob".to_string(),
        &Right::Read,
        &"anyone".to_string(),
    );
    my_database.delegate(
        &Target::Variable("my_var1".to_string()),
        &"bob".to_string(),
        &Right::Write,
        &"anyone".to_string(),
    );
    my_database.delegate(
        &Target::Variable("my_var1".to_string()),
        &"bob".to_string(),
        &Right::Append,
        &"anyone".to_string(),
    );
    my_database.delegate(
        &Target::Variable("my_var1".to_string()),
        &"bob".to_string(),
        &Right::Delegate,
        &"anyone".to_string(),
    );

    let mut my_principal = my_database
        .principals
        .get(&"bob".to_string())
        .cloned()
        .expect("couldnt find my_principal");
    match my_principal {
        VPrincipal::Anyone(ref mut my_delegations) => println!("{:?}", my_delegations.delegations),
        VPrincipal::User(ref mut my_delegations, _) => println!("{:?}", my_delegations.delegations),
        VPrincipal::Admin(_) => println!("got admin"),
    }

    // check for correct permissions
    assert_eq!(
        my_database.check_right(
            &Target::Variable("my_var1".to_string()),
            &Right::Read,
            &"admin".to_string()
        ) && my_database.check_right(
            &Target::Variable("my_var1".to_string()),
            &Right::Write,
            &"admin".to_string()
        ) && my_database.check_right(
            &Target::Variable("my_var1".to_string()),
            &Right::Append,
            &"admin".to_string()
        ) && my_database.check_right(
            &Target::Variable("my_var1".to_string()),
            &Right::Delegate,
            &"admin".to_string()
        ) && my_database.check_right(
            &Target::Variable("my_var1".to_string()),
            &Right::Read,
            &"bob".to_string()
        ) && my_database.check_right(
            &Target::Variable("my_var1".to_string()),
            &Right::Write,
            &"bob".to_string()
        ) && my_database.check_right(
            &Target::Variable("my_var1".to_string()),
            &Right::Append,
            &"bob".to_string()
        ) && my_database.check_right(
            &Target::Variable("my_var1".to_string()),
            &Right::Delegate,
            &"bob".to_string()
        ) && my_database.check_right(
            &Target::Variable("my_var1".to_string()),
            &Right::Read,
            &"anyone".to_string()
        ) && my_database.check_right(
            &Target::Variable("my_var1".to_string()),
            &Right::Write,
            &"anyone".to_string()
        ) && my_database.check_right(
            &Target::Variable("my_var1".to_string()),
            &Right::Append,
            &"anyone".to_string()
        ) && my_database.check_right(
            &Target::Variable("my_var1".to_string()),
            &Right::Delegate,
            &"anyone".to_string()
        ),
        true
    );

    //add principals to database after anyone has some permissions
    my_database.create_principal(&"alice".to_string(), &hash("alice_pass".to_string()));

    Ok(())
}
