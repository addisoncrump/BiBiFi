use std::ops::Not;

/// The first line of a valid program indicates a principal and her password. Each subsequent line
/// contains a primitive command executed on the principal’s behalf.  The server outputs a status
/// code to the client’s connection for each primitive command executed (assuming the whole program
/// completes successfully). A program concludes by either computing and returning some expression
/// or instructing the server to exit. Primitive commands may define, add, or change entities stored
/// at the server, and these entities are visible to subsequent, properly authorized programs.
///
/// Here is a slightly more complicated version of the first example program:
/// ```text
/// as principal admin password "admin" do
///    create principal bob "B0BPWxxd"
///    set x = "my string"
///    set y = { f1 = x, f2 = "field2" }
///    set delegation x admin read -> bob
///    return y.f1
/// ***
/// ```
///
/// The first line indicates that this program is running on behalf of principal admin, whose
/// password is "admin".
///
/// Each subsequent line is a primitive command; each primitive command is executed in order with
/// admin’s authority. This program:
///  - creates a principal bob (and sets bob’s password);
///  - creates a new global variable x and initializes it to "my string";
///  - creates another global variable y and initializes it to a record with two fields, where the
///    first field, named f1, is initialized to x’s value ("my string"), and the second field, named
///    f2, is initialized to the string "field2";
///  - specifies that bob may read x’s contents (by delegating admin’s read authority on x to bob);
///    and
///  - returns the value of y’s f1 field.
///  - finally, the sequence *** signals the definitive end of the program
///
/// The output of running this program is sent back to the client. This output is a sequence of
/// status codes in JSON format, one per command:
/// ```javascript
/// {"status":"CREATE_PRINCIPAL"}
/// {"status":"SET"}
/// {"status":"SET"}
/// {"status":"SET_DELEGATION"}
/// {"status":"RETURNING","output":"my string"}
/// ```
///
/// Notice that the RETURNING status code is coupled with an output field, which has a JSON
/// representation of the returned value (all other status codes have no additional output).
///
/// The created principal (bob) and global variables (x and y) persist and so are available to
/// subsequent programs run on the server, assuming those programs’ running principal is authorized
/// to access the variables. For example, suppose we were to then run the following program:
/// ```text
/// as principal bob password "B0BPWxxd" do
///    return x
/// ***
/// ```
///
/// Because principal bob was granted access to read x, the client should get the following output:
/// ```javascript
/// {"status":"RETURNING","output":"my string"}
/// ```
///
/// The following program would result in a security violation because bob does not have permission
/// to write x, only read it:
/// ```text
/// as principal bob password "B0BPWxxd" do
///    set z = "bobs string"
///    set x = "another string"
///    return x
/// ***
/// ```
///
/// The output of this program would be:
/// ```javascript
/// {"status":"DENIED"}
/// ```
///
/// (We would have gotten the same output had bob tried to access variable y, since he was not
/// delegated any access to it.)
///
/// What about variable z? Programs are transactional, which means that either the entire program
/// succeeds or none of it succeeds.  Thus, as a result of the security violation, the creation of
/// variable z is rolled back so it is as if z was never created and thus subsequent programs will
/// not see it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Program {
    /// The principal executing this program.
    pub principal: Principal,
    /// The password used to authenticate the principal.
    pub password: [u8; 32],
    /// The commands for the program.
    pub commands: Vec<PrimitiveCommand>,
    /// The termination command for the program
    pub terminator: TerminatorCommand,
}

/// Each program is run as a different user, referred to as a principal. Whichever principal runs
/// the program determines what data the program can access. The program in this example is being
/// run by a principal called admin, which is the superuser of the system; we will return to admin’s
/// abilities later.
#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub struct Principal {
    /// The identifier which identifies this principal by name, used for lookups and locking in the
    /// database during runtime.
    pub ident: Identifier,
}

/// A command is a single executable element of the program. See each member for details.
#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub enum TerminatorCommand {
    /// If the command is exit, then the server outputs the status code is EXITING, terminates the
    /// client connection, and halts with return code 0 (and thus does not accept any more
    /// connections). This command is only allowed if the current principal is admin; otherwise it
    /// is a security violation.
    Exit,
    /// If the command is return <expr> then the server executes the expression and outputs status
    /// code RETURNING and the JSON representation of the result for the key "output"; the output
    /// format is given at the end of this document.
    Return(Expr),
}

/// Other than return and exit, a <cmd> is an ordered list of primitive commands separated by
/// newlines; we detail each primitive command below. Note that commands may include expressions;
/// these are executed as discussed [in Expr](enum.Expr.html). If an expression fails or issues a
/// security violation, then the command that invokes it does.
#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub enum PrimitiveCommand {
    /// Creates a principal p having password s.
    ///
    /// The system is preconfigured with principal admin whose password is given by the second
    /// command-line argument; or "admin" if that password is not given. There is also a
    /// preconfigured principal anyone whose initial password is unspecified, and which has no
    /// inherent authority. (See also the description of default delegator, below, for more about
    /// this command, and see the permissions discussion for more on how principal anyone is used.)
    ///
    /// Failure conditions:
    ///  - Fails if p already exists as a principal.
    ///  - Security violation if the current principal is not admin.
    ///
    /// Successful status code: CREATE_PRINCIPAL
    CreatePrincipal(CreatePrincipal),
    /// Changes the principal p’s password to s.
    ///
    /// Failure conditions:
    ///  - Fails if p does not exist
    ///  - Security violation if the current principal is neither admin nor p itself.
    ///
    /// Successful status code: CHANGE_PASSWORD
    ChangePassword(ChangePassword),
    /// Sets x’s value to the result of evaluating <expr>, where x is a global or local variable.
    /// If x does not exist this command creates it as a global.  If x is created by this command,
    /// and the current principal is not admin, then the current principal is delegated read, write,
    /// append, and delegate rights from the admin on x (equivalent to executing set delegation x
    /// admin read -> p and set delegation x admin write -> p, etc. where p is the current
    /// principal).
    ///
    /// Setting a variable results in a “deep copy.”  For example, consider the following program:
    ///
    /// ```text
    /// set x = "hello"
    /// set y = x
    /// set x = "there"
    /// ```
    ///
    /// At this point, y is still "hello" even though we have re-set x.
    ///
    /// Failure conditions:
    ///  - May fail or have a security violation due to evaluating <expr>
    ///  - Security violation if the current principal does not have write permission on x.
    ///
    /// Successful status code: SET
    Assignment(Assignment),
    /// Adds the <expr>’s result to the end of x. If <expr> evaluates to a record or a string, it is
    /// added to the end of x; if <expr> evaluates to a list, then it is concatenated to (the end
    /// of) x.
    ///
    /// Failure conditions:
    ///  - Fails if x is not defined
    ///  - Security violation if the current principal does not have either write or append
    ///    permission on x (read permission is not necessary).
    ///  - Fails if x is not a list
    ///  - May fail or have a security violation due to evaluating <expr>
    ///
    /// Successful status code: APPEND
    Append(Append),
    /// Creates a local variable x and initializes it to the value of executing <expr>. Subsequent
    /// updates to x can be made as you would to a global variable, e.g., using set x, append...to
    /// x, foreach, etc. as described elsewhere in this section. Different from a global variable,
    /// local variables are destroyed when the program ends—they do not persist across connections.
    ///
    /// Failure conditions:
    ///  - Fails if x is already defined as a local or global variable.
    ///  - May fail or have a security violation due to evaluating <expr>
    ///
    /// Successful status code: LOCAL
    LocalAssignment(Assignment),
    /// For each element y in list x, replace the contents of y with the result of executing <expr>.
    /// This expression is called for each element in x, in order, and y is bound to the current
    /// element.
    ///
    /// As an example, consider the following program:
    ///
    /// ```text
    /// as principal admin password "admin" do
    ///    set records = []
    ///    append to records with { name = "mike", date = "1-1-90" }
    ///    append to records with { name = "dave", date = "1-1-85" }
    ///    local names = records
    ///    foreach rec in names replacewith rec.name
    ///    return names
    /// ***
    /// ```
    ///
    /// This program creates a list of two records, each with fields name and date. Then it makes a
    /// copy of this list in names, and updates the contents of names using foreach. This foreach
    /// iterates over the list and replaces each record with the first element of that record. So
    /// the final, returned variable names is ["mike","dave"]. (The original list records is not
    /// changed by the foreach here.)
    ///
    /// Failure conditions:
    ///  - Fails if x is not defined
    ///  - Security violation if the current principal does not have read and write permission on x.
    ///  - Fails if y is already defined as a local or global variable.
    ///  - Fails if x is not a list.
    ///  - If any execution of <expr> fails or has a security violation, then entire foreach does.
    ///    <expr> is from the front of the list to the back
    ///  - Fails if <expr> evaluates to a list, rather than a string or a record.
    ///
    /// Successful status code: FOREACH
    ForEach(ForEach),
    /// When <tgt> is a variable x, Indicates that q delegates <right> to p on x, so that p is given
    /// <right> whenever q is. If p is anyone, then effectively all principals are given <right> on
    /// x (for more detail, see here). When <tgt> is the keyword all then q delegates <right> to p
    /// for all variables on which q (currently) has delegate permission.
    ///
    /// Failure conditions:
    ///  - Fails if either p or q does not exist
    ///  - Fails if x does not exist or if it is a local variable, if <tgt> is a variable x.
    ///  - Security violation unless the current principal is admin or q; if the principal is q and
    ///    <tgt> is the variable x, then q must have delegate permission on x.
    ///
    /// Successful status code: SET_DELEGATION
    SetDelegation(Delegation),
    /// When <tgt> is a variable x, indicates that q revokes a delegation assertion of <right> to p
    /// on x. In effect, this command revokes a previous command set delegation x q <right> -> p;
    /// see below for the precise semantics of what this means. If <tgt> is the keyword all then q
    /// revokes delegation of <right> to p for all variables on which q has delegate permission.
    ///
    /// Failure conditions:
    ///  - Fails if either p or q does not exist
    ///  - Fails if x does not exist or if it is a local variable, if <tgt> is a variable x.
    ///  - Security violation unless the current principal is admin, p, or q; if the principal is q
    ///    and <tgt> is a variable x, then it must have delegate permission on x. (No special
    ///    permission is needed if the current principal is p: any non-admin principal can always
    ///    deny himself rights).
    ///
    /// Successful status code: DELETE_DELEGATION
    DeleteDelegation(Delegation),
    /// Sets the “default delegator” to p. This means that when a principal q is created, the system
    /// automatically delegates all from p to q. Changing the default delegator does not affect the
    /// permissions of existing principals. The initial default delegator is anyone.
    ///
    /// Failure conditions:
    ///  - Fails if p does not exist
    ///  - Security violation if the current principal is not admin.
    ///
    /// Successful status code: DEFAULT_DELEGATOR
    DefaultDelegator(Principal),
}

/// The struct containing the data required to represent the
/// [CreatePrincipal](enum.PrimitiveCommand.html#variant.CreatePrincipal) primitive command.
#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub struct CreatePrincipal {
    /// The principal to be created.
    pub principal: Principal,
    /// The password to be used for the principal.
    pub password: [u8; 32],
}

/// The struct containing the data required to represent the
/// [ChangePassword](enum.PrimitiveCommand.html#variant.ChangePassword) primitive command.
#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub struct ChangePassword {
    /// The principal who's password will be changed.
    pub principal: Principal,
    /// The password to set it to (hashed).
    pub password: [u8; 32],
}

#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub struct Assignment {
    pub variable: Variable,
    pub expr: Expr,
}

#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub struct Append {
    pub variable: Variable,
    pub expr: Expr,
}

#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub struct ForEach {
    pub value: Variable,
    pub list: Variable,
    pub expr: Expr,
}

#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub struct Delegation {
    pub target: Target,
    pub delegator: Principal,
    pub right: Right,
    pub delegated: Principal,
}

#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub enum Expr {
    Value(Value),
    EmptyList,
    FieldVals(Vec<Assignment>),
}

#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub enum Value {
    Variable(Variable),
    String(String),
}

#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub enum Variable {
    Variable(Identifier),
    Member(Identifier, Box<Variable>), // nested values possible, but not implemented
}

#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub enum Type {
    Root,
    List,
    Member,
}

#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub enum Target {
    All,
    Variable(Identifier),
}

#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub enum Right {
    Read,
    Write,
    Append,
    Delegate,
}

#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub enum Scope {
    Local,
    Global,
}

impl Not for Scope {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Scope::Local => Scope::Global,
            Scope::Global => Scope::Local,
        }
    }
}

#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub struct Identifier {
    pub name: String,
}
