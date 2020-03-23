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

    // check for correct permissions
    assert_eq!(
        my_database.check_right(&Target::Variable("my_var1".to_string()), &Right::Read, &"alice".to_string()) &&
            my_database.check_right(&Target::Variable("my_var1".to_string()), &Right::Write, &"alice".to_string()) &&
            my_database.check_right(&Target::Variable("my_var1".to_string()), &Right::Append, &"alice".to_string()) &&
            my_database.check_right(&Target::Variable("my_var1".to_string()), &Right::Delegate, &"alice".to_string()),
        true
    );

    //alice created my_var2
    my_database.delegate(&Target::Variable("my_var2".to_string()), &"admin".to_string(), &Right::Read, &"alice".to_string());
    my_database.delegate(&Target::Variable("my_var2".to_string()), &"admin".to_string(), &Right::Write, &"alice".to_string());
    my_database.delegate(&Target::Variable("my_var2".to_string()), &"admin".to_string(), &Right::Append, &"alice".to_string());
    my_database.delegate(&Target::Variable("my_var2".to_string()), &"admin".to_string(), &Right::Delegate, &"alice".to_string());

    ////change the default


    ////change bob's password
    //my_database.change_password(&"bob".to_string(), &hash("bob_new_pass".to_string()) )
    //assert_eq!(
    //    my_database.check_pass(&"bob".to_string(),&hash("bob_new_pass".to_string())) &&
    //        !my_database.check_pass(&"bob".to_string(),&hash("bob_pass".to_string())),
    //    true
    //);

    ////add principals to database after new default alice has some permissions other than prev default anyone
    //my_database.create_principal(&"john".to_string(), &hash("john_pass".to_string()) );
    //// check for correct permissions
    //assert_eq!(
    //    my_database.check_right(&Target::Variable("my_var2".to_string()), &Right::Read, &"john".to_string()) &&
    //        my_database.check_right(&Target::Variable("my_var2".to_string()), &Right::Write, &"john".to_string()) &&
    //        my_database.check_right(&Target::Variable("my_var2".to_string()), &Right::Append, &"john".to_string()) &&
    //        my_database.check_right(&Target::Variable("my_var2".to_string()), &Right::Delegate, &"john".to_string()),
    //    true
    //);



    Ok(())
}
