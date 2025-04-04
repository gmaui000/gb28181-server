use constructor::{Get, New, Set};

#[derive(Set, Get, New, Default, Debug, PartialEq)]
pub struct Foo {
    a: i32,
    b: String,
    c: bool,
    d: Option<u32>,
}

#[test]
fn test_foo() {
    let mut foo = Foo::new(112i32, String::from("abc"), true, Some(12u32));
    let rf = Foo {
        a: 112,
        b: "abc".to_string(),
        c: true,
        d: Some(12u32),
    };
    assert_eq!(&foo, &rf);
    foo.set_a(456);
    foo.set_b("bb".to_string());
    assert_eq!(foo.get_a(), &456i32);
    assert_eq!(foo.get_b(), &"bb".to_string());
}

#[derive(Set, Get, New, Default, Debug, PartialEq)]
#[set(a, b, c)]
#[get(b, c)]
#[new(b, c)]
pub struct Bar {
    a: i32,
    b: String,
    c: bool,
    d: f32,
}

#[test]
fn test_bar() {
    let mut bar = Bar::new("bbb".to_string(), true);
    let rb = Bar {
        a: 0,
        b: String::from("bbb"),
        c: true,
        d: 0.0f32,
    };
    assert_eq!(&bar, &rb);
    bar.set_a(111);
    bar.set_b("nb");
    bar.set_c(false);
    assert_eq!(bar.get_b(), &"nb".to_string());
    assert!(!*bar.get_c());
}

#[derive(Set, Get, New, Default, Debug, PartialEq)]
pub struct UnFoo(u32, String, bool);

#[test]
fn test_un_foo() {
    let mut un_foo = UnFoo::new(1u32, "sss".to_string(), true);
    let ruf = UnFoo(1u32, "sss".to_string(), true);
    assert_eq!(&un_foo, &ruf);
    un_foo.set_0(10u32);
    un_foo.set_1("xxx".to_string());
    un_foo.set_2(false);
    assert_eq!(un_foo.get_0(), &10u32);
    assert_eq!(un_foo.get_1(), &"xxx".to_string());
    assert_eq!(un_foo.get_2(), &false);
}

#[derive(Set, Get, New, Default, Debug, PartialEq)]
#[set(0, 2)]
#[get(0, 2)]
#[new(0, 2)]
pub struct UnBar(u32, String, bool, i32);

#[test]
fn test_un_bar() {
    let mut un_bar = UnBar::new(234u32, true);
    let rub = UnBar(234u32, "".to_string(), true, 0);
    assert_eq!(&un_bar, &rub);
    un_bar.set_0(111u32);
    un_bar.set_2(false);
    assert_eq!(un_bar.get_0(), &111u32);
    assert_eq!(un_bar.get_2(), &false);
}
