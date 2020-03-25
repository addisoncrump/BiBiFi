use super::*;
use std::error::Error;

use bibifi_util::hash;

#[test]
// test all basic stuff
fn basic_full_1() -> Result<(), Box<dyn Error>> {
    let mut my_database = Database::new(hash("wolla".to_string()));

    //admin with correct password checks true
    assert_eq!(my_database.check_pass(&"admin".to_string(), &hash("wolla".to_string())), SUCCESS);

    //admin with wrong password checks false
    assert_eq!(my_database.check_pass(&"admin".to_string(), &hash("wollabig".to_string())), DENIED);

    //anyone principal rejected with any password
    assert_eq!(my_database.check_pass(&"anyone".to_string(), &hash("".to_string())), DENIED);

    //check non admin principal
    assert_eq!(my_database.check_pass(&"not_admin".to_string(), &hash("wolla".to_string())), FAILED);

    //add principals to database
    my_database.create_principal(
        &"admin".to_string(),
        &"bob".to_string(),
        &hash("".to_string()),
    ); // empty string password
    my_database.create_principal(
        &"admin".to_string(),
        &"tom".to_string(),
        &hash("tom_pass".to_string()),
    );

    //principal with correct password checks true
    assert_eq!(my_database.check_pass(&"bob".to_string(), &hash("".to_string())), SUCCESS);
    assert_eq!(my_database.check_pass(&"tom".to_string(), &hash("tom_pass".to_string())), SUCCESS);

    //principal with wrong password checks false
    assert_eq!(my_database.check_pass(&"bob".to_string(), &hash("wolla".to_string())), DENIED);
    assert_eq!(my_database.check_pass(&"tom".to_string(), &hash("".to_string())), DENIED);
    assert_eq!(my_database.check_pass(&"tom".to_string(), &hash("tom".to_string())), DENIED);

    // lets say, bob created my_var and delegated all permissions to everyone
    my_database.set(&"bob".to_string(), &"my_var1".to_string(), &Value::Immediate("lmao".to_string()));
    // rights delegated automatically, hopefully

    my_database.delegate(
        &"admin".to_string(),
        &Target::Variable("my_var1".to_string()),
        &"bob".to_string(),
        &Right::Read,
        &"anyone".to_string(),
    );
    my_database.delegate(
        &"admin".to_string(),
        &Target::Variable("my_var1".to_string()),
        &"bob".to_string(),
        &Right::Write,
        &"anyone".to_string(),
    );
    my_database.delegate(
        &"admin".to_string(),
        &Target::Variable("my_var1".to_string()),
        &"bob".to_string(),
        &Right::Append,
        &"anyone".to_string(),
    );
    my_database.delegate(
        &"admin".to_string(),
        &Target::Variable("my_var1".to_string()),
        &"bob".to_string(),
        &Right::Delegate,
        &"anyone".to_string(),
    );

    // check for correct permissions
    assert_eq!(my_database.check_right(&"my_var1".to_string(), &Right::Read, &"admin".to_string()), true);
    assert_eq!(my_database.check_right(&"my_var1".to_string(), &Right::Write, &"admin".to_string()), true);
    assert_eq!(my_database.check_right(&"my_var1".to_string(), &Right::Append, &"admin".to_string()), true);
    assert_eq!(my_database.check_right(&"my_var1".to_string(), &Right::Delegate, &"admin".to_string()), true);
    assert_eq!(my_database.check_right(&"my_var1".to_string(), &Right::Read, &"bob".to_string()), true);
    assert_eq!(my_database.check_right(&"my_var1".to_string(), &Right::Write, &"bob".to_string()), true);
    assert_eq!(my_database.check_right(&"my_var1".to_string(), &Right::Append, &"bob".to_string()), true);
    assert_eq!(my_database.check_right(&"my_var1".to_string(), &Right::Delegate, &"bob".to_string()), true);
    assert_eq!(my_database.check_right(&"my_var1".to_string(), &Right::Read, &"anyone".to_string()), true);
    assert_eq!(my_database.check_right(&"my_var1".to_string(), &Right::Write, &"anyone".to_string()), true);
    assert_eq!(my_database.check_right(&"my_var1".to_string(), &Right::Append, &"anyone".to_string()), true);
    assert_eq!(my_database.check_right(&"my_var1".to_string(), &Right::Delegate, &"anyone".to_string()), true);

    //add principals to database after anyone has some permissions
    my_database.create_principal(&"admin".to_string(), &"alice".to_string(), &hash("alice_pass".to_string()));

    // check for correct permissions
    assert_eq!(my_database.check_right(&"my_var1".to_string(), &Right::Read, &"alice".to_string()), true);
    assert_eq!(my_database.check_right(&"my_var1".to_string(), &Right::Write, &"alice".to_string()), true);
    assert_eq!(my_database.check_right(&"my_var1".to_string(), &Right::Append, &"alice".to_string()), true);
    assert_eq!(my_database.check_right(&"my_var1".to_string(), &Right::Delegate, &"alice".to_string()), true);


    //alice created my_var2
    my_database.set(&"bob".to_string(), &"my_var2".to_string(), &Value::Immediate("lmao".to_string()));

    //change the default
    my_database.set_default_delegator(&"admin".to_string(), &"alice".to_string());

    //change bob's password
    my_database.change_password(&"bob".to_string(), &"bob".to_string(), &hash("bob_new_pass".to_string()));
    assert_eq!(my_database.check_pass(&"bob".to_string(), &hash("bob_new_pass".to_string())), SUCCESS);
    assert_eq!(my_database.check_pass(&"bob".to_string(), &hash("bob_pass".to_string())), DENIED);

    Ok(())
}

#[test]
// test all basic stuff
fn basic_full_2() -> Result<(), Box<dyn Error>> {
    let mut my_database = Database::new(hash("wolla".to_string()));

    //add principals to database
    my_database.create_principal(&"user".to_string(), &"bob".to_string(), &hash("".to_string())); // empty string password
    my_database.create_principal(&"user".to_string(), &"tom".to_string(), &hash("tom_pass".to_string()));

    // lets say, bob created my_var and delegated all permissions to everyone
    my_database.set(&"bob".to_string(), &"my_var1".to_string(), &Value::Immediate("lmao".to_string()));

    my_database.delegate(
        &"admin".to_string(),
        &Target::Variable("my_var1".to_string()),
        &"bob".to_string(),
        &Right::Read,
        &"anyone".to_string(),
    );
    my_database.delegate(
        &"admin".to_string(),
        &Target::Variable("my_var1".to_string()),
        &"bob".to_string(),
        &Right::Write,
        &"anyone".to_string(),
    );
    my_database.delegate(
        &"admin".to_string(),
        &Target::Variable("my_var1".to_string()),
        &"bob".to_string(),
        &Right::Append,
        &"anyone".to_string(),
    );
    my_database.delegate(
        &"admin".to_string(),
        &Target::Variable("my_var1".to_string()),
        &"bob".to_string(),
        &Right::Delegate,
        &"anyone".to_string(),
    );

    //add principals to database after anyone has some permissions
    my_database.create_principal(&"admin".to_string(), &"alice".to_string(), &hash("alice_pass".to_string()));

    //alice created my_var2
    my_database.set(&"alice".to_string(), &"my_var2".to_string(), &Value::Immediate("lmao".to_string()));

    //change the default
    my_database.set_default_delegator(&"admin".to_string(), &"alice".to_string());

    //change bob's password
    my_database.change_password(&"bob".to_string(), &"bob".to_string(), &hash("bob_new_pass".to_string()));

    //add principals to database after new default alice has some permissions other than prev default anyone
    my_database.create_principal(&"admin".to_string(), &"john".to_string(), &hash("john_pass".to_string()));

    // check for correct permissions
    assert_eq!(my_database.check_right(&"my_var2".to_string(), &Right::Read, &"john".to_string()), true);
    assert_eq!(my_database.check_right(&"my_var2".to_string(), &Right::Write, &"john".to_string()), true);
    assert_eq!(my_database.check_right(&"my_var2".to_string(), &Right::Append, &"john".to_string()), true);
    assert_eq!(my_database.check_right(&"my_var2".to_string(), &Right::Delegate, &"john".to_string()), true);


    //alice created my_var3
    my_database.set(&"alice".to_string(), &"my_var3".to_string(), &Value::Immediate("lmao".to_string()));


    //add principals to database after new default alice has some permissions other than prev default anyone
    my_database.create_principal(&"admin".to_string(), &"git".to_string(), &hash("git_pass".to_string()));
    // check for in-correct permissions
    assert_eq!(my_database.check_right(&"my_var3".to_string(), &Right::Read, &"git".to_string()), true);
    assert_eq!(my_database.check_right(&"my_var3".to_string(), &Right::Write, &"git".to_string()), true);
    assert_eq!(my_database.check_right(&"my_var3".to_string(), &Right::Append, &"git".to_string()), true);
    assert_eq!(my_database.check_right(&"my_var3".to_string(), &Right::Delegate, &"git".to_string()), true);


    my_database.create_principal(&"admin".to_string(), &"git1".to_string(), &hash("git_pass".to_string()));

    //alice created my_var4
    my_database.set(&"alice".to_string(), &"my_var4".to_string(), &Value::Immediate("lmao".to_string()));


    // check for in-correct permissions
    assert_eq!(my_database.check_right(&"my_var4".to_string(), &Right::Read, &"git1".to_string()), false);
    assert_eq!(my_database.check_right(&"my_var4".to_string(), &Right::Write, &"git1".to_string()), false);
    assert_eq!(my_database.check_right(&"my_var4".to_string(), &Right::Append, &"git1".to_string()), false);
    assert_eq!(my_database.check_right(&"my_var4".to_string(), &Right::Delegate, &"git1".to_string()), false);

    Ok(())
}
