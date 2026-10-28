#![forbid(unsafe_code)]
use core::cmp::Ordering;
use core::sync::atomic::{AtomicU64, Ordering as Memory};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Overflow;
pub type R<T> = Result<T, Overflow>;

static WORK: AtomicU64 = AtomicU64::new(0);
pub fn tick(units: u64) { WORK.fetch_add(units, Memory::Relaxed); }
pub fn work() -> u64 { WORK.load(Memory::Relaxed) }

pub fn nat_add(a: u64, b: u64) -> R<u64> { a.checked_add(b).ok_or(Overflow) }
pub fn nat_sub(a: u64, b: u64) -> u64 { a.saturating_sub(b) }
pub fn nat_mul(a: u64, b: u64) -> R<u64> { a.checked_mul(b).ok_or(Overflow) }
pub fn nat_quot(a: u64, b: u64, z: u64) -> u64 { a.checked_div(b).unwrap_or(z) }
pub fn nat_rem(a: u64, b: u64, z: u64) -> u64 { a.checked_rem(b).unwrap_or(z) }
pub fn nat_eq(a: u64, b: u64) -> bool { a == b }
pub fn nat_le(a: u64, b: u64) -> bool { a <= b }
pub fn nat_lt(a: u64, b: u64) -> bool { a < b }
pub fn nat_succ(a: u64) -> R<u64> { a.checked_add(1).ok_or(Overflow) }
pub fn int_add(a: i64, b: i64) -> R<i64> { a.checked_add(b).ok_or(Overflow) }
pub fn int_sub(a: i64, b: i64) -> R<i64> { a.checked_sub(b).ok_or(Overflow) }
pub fn int_mul(a: i64, b: i64) -> R<i64> { a.checked_mul(b).ok_or(Overflow) }
pub fn int_neg(a: i64) -> R<i64> { a.checked_neg().ok_or(Overflow) }
pub fn int_quot(a: i64, b: i64, z: i64) -> R<i64> { if b == 0 { Ok(z) } else { a.checked_div(b).ok_or(Overflow) } }
pub fn int_rem(a: i64, b: i64, z: i64) -> i64 { if b == 0 { z } else { a.wrapping_rem(b) } }
pub fn bool_not(a: bool) -> bool { !a }
pub fn bool_and(a: bool, b: bool) -> bool { a && b }
pub fn bool_or(a: bool, b: bool) -> bool { a || b }

pub trait Same { fn same(&self, other: &Self) -> bool; }
pub trait Key { fn key(&self, other: &Self) -> Ordering; }
macro_rules! scalar {
    ($($t:ty),*) => { $(
        impl Same for $t { fn same(&self, other: &Self) -> bool { self == other } }
        impl Key for $t { fn key(&self, other: &Self) -> Ordering { self.cmp(other) } }
    )* };
}
scalar!(bool, u8, u16, u32, u64, i8, i16, i32, i64);
impl Same for Ordering { fn same(&self, other: &Self) -> bool { self == other } }
impl<A: Key, B: Key> Key for (A, B) {
    fn key(&self, other: &Self) -> Ordering {
        match self.0.key(&other.0) { Ordering::Equal => self.1.key(&other.1), decided => decided }
    }
}
pub fn equal<T: Same>(a: T, b: T) -> bool { a.same(&b) }
pub fn compare<T: Key>(a: T, b: T) -> Ordering { a.key(&b) }

macro_rules! fixed {
    ($m:ident, $t:ident) => {
        pub mod $m {
            pub fn checked_add(a: $t, b: $t) -> Option<$t> { a.checked_add(b) }
            pub fn checked_sub(a: $t, b: $t) -> Option<$t> { a.checked_sub(b) }
            pub fn checked_mul(a: $t, b: $t) -> Option<$t> { a.checked_mul(b) }
            pub fn checked_quot(a: $t, b: $t) -> Option<$t> { a.checked_div(b) }
            pub fn bit_and(a: $t, b: $t) -> $t { a & b }
            pub fn bit_or(a: $t, b: $t) -> $t { a | b }
            pub fn bit_xor(a: $t, b: $t) -> $t { a ^ b }
            pub fn bit_not(a: $t) -> $t { !a }
            pub fn shift_left(a: $t, amount: u32) -> Option<$t> { if amount < $t::BITS { Some(a.wrapping_shl(amount)) } else { None } }
            pub fn shift_right(a: $t, amount: u32) -> Option<$t> { if amount < $t::BITS { Some(a.wrapping_shr(amount)) } else { None } }
            pub fn convert(a: i128) -> Option<$t> { $t::try_from(a).ok() }
        }
    };
}
fixed!(fixed_u8, u8); fixed!(fixed_u16, u16); fixed!(fixed_u32, u32); fixed!(fixed_u64, u64);
fixed!(fixed_i8, i8); fixed!(fixed_i16, i16); fixed!(fixed_i32, i32); fixed!(fixed_i64, i64);
pub fn checked_neg_i8(a: i8) -> Option<i8> { a.checked_neg() }
pub fn checked_neg_i16(a: i16) -> Option<i16> { a.checked_neg() }
pub fn checked_neg_i32(a: i32) -> Option<i32> { a.checked_neg() }
pub fn checked_neg_i64(a: i64) -> Option<i64> { a.checked_neg() }

#[derive(Clone)]
pub struct Str(std::rc::Rc<str>);
impl Str {
    pub fn lit(text: &str) -> Str { Str(std::rc::Rc::from(text)) }
    pub fn text(&self) -> &str { &self.0 }
}
#[derive(Clone)]
pub struct Bytes(std::rc::Rc<[u8]>);
impl Bytes {
    pub fn lit(octets: &[u8]) -> Bytes { Bytes(std::rc::Rc::from(octets)) }
    pub fn octets(&self) -> &[u8] { &self.0 }
}

pub struct Node<T> { head: T, tail: List<T> }
/// A list is a run of cells laid out contiguously, from the newest at the end of a buffer
/// to the oldest at its start, or a cell in front of another list. A `cons` onto a run that
/// owns its buffer alone pushes in place; a `cons` onto any other list is one cell, so
/// building and taking apart a cell is constant work whatever the sharing, and no buffer is
/// written while another list can see it.
pub enum Repr<T> { Nil, Run { buffer: std::rc::Rc<Vec<T>>, len: usize }, Cell(std::rc::Rc<Node<T>>) }
pub struct List<T>(Repr<T>);
impl<T> Clone for List<T> {
    fn clone(&self) -> Self {
        List(match &self.0 {
            Repr::Nil => Repr::Nil,
            Repr::Run { buffer, len } => Repr::Run { buffer: buffer.clone(), len: *len },
            Repr::Cell(cell) => Repr::Cell(cell.clone()),
        })
    }
}
impl<T> List<T> {
    fn take(&mut self) -> Repr<T> { std::mem::replace(&mut self.0, Repr::Nil) }
}
impl<T> Drop for Node<T> {
    fn drop(&mut self) {
        let mut next = match self.tail.take() { Repr::Cell(cell) => Some(cell), _ => None };
        while let Some(cell) = next { // release
            match std::rc::Rc::try_unwrap(cell) {
                Ok(mut node) => next = match node.tail.take() { Repr::Cell(cell) => Some(cell), _ => None },
                Err(_) => break,
            }
        }
    }
}
impl<T: Clone> List<T> {
    pub fn nil() -> Self { List(Repr::Nil) }
    pub fn cons(head: T, mut tail: List<T>) -> Self {
        List(match tail.take() {
            Repr::Nil => {
                let mut buffer = Vec::with_capacity(16);
                buffer.push(head);
                Repr::Run { buffer: std::rc::Rc::new(buffer), len: 1 }
            }
            Repr::Run { mut buffer, len } => {
                if let Some(cells) = std::rc::Rc::get_mut(&mut buffer) {
                    cells.truncate(len);
                    cells.push(head);
                    Repr::Run { buffer, len: len + 1 }
                } else {
                    Repr::Cell(std::rc::Rc::new(Node { head, tail: List(Repr::Run { buffer, len }) }))
                }
            }
            rest => Repr::Cell(std::rc::Rc::new(Node { head, tail: List(rest) })),
        })
    }
    pub fn uncons(&self) -> Option<(T, List<T>)> {
        match &self.0 {
            Repr::Nil => None,
            Repr::Run { buffer, len } => {
                let head = buffer[len - 1].clone();
                Some((head, List(if *len == 1 { Repr::Nil } else { Repr::Run { buffer: buffer.clone(), len: len - 1 } })))
            }
            Repr::Cell(cell) => Some((cell.head.clone(), cell.tail.clone())),
        }
    }
    /// Walk the items in order, ticking one per item, until `each` says to stop: the cells
    /// in front, then the run they end in.
    fn walk(&self, each: &mut dyn FnMut(&T) -> bool) {
        let mut cursor = self;
        while let Repr::Cell(cell) = &cursor.0 { tick(1); if !each(&cell.head) { return; } cursor = &cell.tail; }
        if let Repr::Run { buffer, len } = &cursor.0 {
            for item in buffer[..*len].iter().rev() { tick(1); if !each(item) { return; } }
        }
    }
    fn items(&self) -> Vec<T> {
        let mut out = Vec::new();
        self.walk(&mut |item| { out.push(item.clone()); true });
        out
    }
    fn onto(items: Vec<T>, tail: List<T>) -> List<T> {
        let mut out = tail;
        for item in items.into_iter().rev() { tick(1); out = List::cons(item, out); }
        out
    }
}

fn chars(text: &str) -> u64 {
    let mut count = 0;
    for _ in text.chars() { tick(1); count += 1; }
    count
}

impl Same for Str {
    fn same(&self, other: &Self) -> bool {
        let mut left = self.0.chars();
        let mut right = other.0.chars();
        loop {
            tick(1);
            match (left.next(), right.next()) { (None, None) => return true, (a, b) if a != b => return false, _ => {} }
        }
    }
}
impl Same for Bytes {
    fn same(&self, other: &Self) -> bool { tick(1 + self.0.len().min(other.0.len()) as u64); self.0 == other.0 }
}
impl Key for Str {
    fn key(&self, other: &Self) -> Ordering {
        let mut left = self.0.chars();
        let mut right = other.0.chars();
        loop {
            tick(1);
            match (left.next(), right.next()) {
                (None, None) => return Ordering::Equal,
                (None, Some(_)) => return Ordering::Less,
                (Some(_), None) => return Ordering::Greater,
                (Some(a), Some(b)) => match a.cmp(&b) { Ordering::Equal => {} decided => return decided },
            }
        }
    }
}

pub fn append_list<T: Clone>(a: List<T>, b: List<T>) -> List<T> { List::onto(a.items(), b) }
pub fn append_bytes(a: Bytes, b: Bytes) -> Bytes {
    tick((a.0.len() + b.0.len()) as u64);
    let mut out = Vec::with_capacity(a.0.len() + b.0.len());
    out.extend_from_slice(&a.0);
    out.extend_from_slice(&b.0);
    Bytes(std::rc::Rc::from(out))
}
pub fn length_list<T: Clone>(a: List<T>) -> u64 {
    let mut count = 0;
    a.walk(&mut |_| { count += 1; true });
    count
}
pub fn length_bytes(a: Bytes) -> u64 { a.0.len() as u64 }
pub fn length_string(a: Str) -> u64 { chars(&a.0) }
pub fn index_list<T: Clone>(a: List<T>, i: u64) -> Option<T> {
    let mut position = 0;
    let mut found = None;
    a.walk(&mut |item| { if position == i { found = Some(item.clone()); return false; } position += 1; true });
    found
}
pub fn index_bytes(a: Bytes, i: u64) -> Option<u8> { usize::try_from(i).ok().and_then(|i| a.0.get(i)).copied() }
pub fn slice_list<T: Clone>(a: List<T>, start: u64, count: u64) -> Option<List<T>> {
    let end = u128::from(start) + u128::from(count);
    let mut part = Vec::new();
    let mut position: u128 = 0;
    a.walk(&mut |item| {
        if position >= end { return false; }
        if position >= u128::from(start) { part.push(item.clone()); }
        position += 1;
        true
    });
    if position < end { None } else { Some(List::onto(part, List::nil())) }
}
pub fn slice_bytes(a: Bytes, start: u64, count: u64) -> Option<Bytes> {
    let end = u128::from(start) + u128::from(count);
    if end > a.0.len() as u128 { return None; }
    tick(count);
    Some(Bytes::lit(&a.0[start as usize..end as usize]))
}
pub fn utf8_encode(a: Str) -> Bytes { tick(a.0.len() as u64); Bytes::lit(a.0.as_bytes()) }
pub fn utf8_decode(a: Bytes) -> Option<Str> { tick(a.0.len() as u64); core::str::from_utf8(&a.0).ok().map(Str::lit) }
pub fn compare_bytes(a: Bytes, b: Bytes) -> Ordering { tick(1 + a.0.len().min(b.0.len()) as u64); a.0.cmp(&b.0) }
pub fn split_exact(text: Str, delimiter: Str, maximum: u32) -> Option<List<Str>> {
    if delimiter.0.is_empty() { return None; }
    chars(&text.0);
    let fields: Vec<Str> = text.0.split(&*delimiter.0).map(Str::lit).collect();
    if fields.len() as u128 > u128::from(maximum) { return None; }
    Some(List::onto(fields, List::nil()))
}
pub fn join(texts: List<Str>, delimiter: Str) -> Str {
    let mut out = String::new();
    let mut first = true;
    texts.walk(&mut |text| {
        if !first { chars(&delimiter.0); out.push_str(&delimiter.0); }
        first = false;
        chars(&text.0);
        out.push_str(&text.0);
        true
    });
    Str::lit(&out)
}
fn canonical_decimal(text: &str) -> Option<Option<i128>> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    let mut canonical = !digits.is_empty() && (digits == "0" || !digits.starts_with('0')) && text != "-0";
    for c in digits.chars() { tick(1); canonical = canonical && c.is_ascii_digit(); }
    if !canonical { return None; }
    Some(text.parse::<i128>().ok())
}
pub fn parse_int(text: Str) -> R<Option<i64>> {
    match canonical_decimal(&text.0) {
        None => Ok(None),
        Some(Some(n)) => i64::try_from(n).map(Some).map_err(|_| Overflow),
        Some(None) => Err(Overflow),
    }
}
macro_rules! decimal {
    ($t:ident, $format:ident, $parse:ident) => {
        pub fn $format(a: $t) -> Str { let text = a.to_string(); tick(text.len() as u64); Str::lit(&text) }
        pub fn $parse(text: Str) -> Option<$t> {
            match canonical_decimal(&text.0) { Some(Some(n)) => $t::try_from(n).ok(), _ => None }
        }
    };
}
decimal!(u8, format_u8, parse_u8); decimal!(u16, format_u16, parse_u16);
decimal!(u32, format_u32, parse_u32); decimal!(u64, format_u64, parse_u64);
decimal!(i8, format_i8, parse_i8); decimal!(i16, format_i16, parse_i16);
decimal!(i32, format_i32, parse_i32); decimal!(i64, format_i64, parse_i64);

#[derive(Clone)]
pub enum Adt0 {
    C0(Adt1, u32, u64, Bytes, List<Adt1>),
}

#[derive(Clone)]
pub enum Adt1 {
    C0(u32, u32, u32, u32, u32, u32, u32, u32),
}

#[derive(Clone)]
pub enum Adt2 {
    C0(Adt1, Adt3, u64, u32, u32),
}

#[derive(Clone)]
pub enum Adt3 {
    C0(u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32),
}

#[derive(Clone)]
pub enum Adt4 {
    C0(u32, u32, u32, u32),
}

#[derive(Clone)]
pub enum Fn0 {
    F9(u32, Bytes, Adt1, Adt0),
}

impl Fn0 {
    pub fn apply(&self, p0: (u64, List<Adt1>)) -> R<(u64, List<Adt1>)> {
        match self {
            Fn0::F9(k0, k1, k2, k3) => f9(*k0, k1.clone(), k2.clone(), k3.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn1 {
    F15(Adt2),
}

impl Fn1 {
    pub fn apply(&self, p0: (u64, Bytes)) -> R<(u64, Bytes)> {
        match self {
            Fn1::F15(k0) => f15(k0.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn2 {
    F17(u32, Adt1),
}

impl Fn2 {
    pub fn apply(&self, p0: Adt2, p1: Adt1) -> Adt2 {
        match self {
            Fn2::F17(k0, k1) => f17(*k0, k1.clone(), p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn3 {
    F29(u32, u64, u32, u64, u32, Bytes),
}

impl Fn3 {
    pub fn apply(&self, p0: (u64, Adt1)) -> R<(u64, Adt1)> {
        match self {
            Fn3::F29(k0, k1, k2, k3, k4, k5) => f29(*k0, *k1, *k2, *k3, *k4, k5.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn4 {
    F35(u32, Adt1),
}

impl Fn4 {
    pub fn apply(&self, p0: (Adt1, (u64, List<Adt1>))) -> Option<(Adt1, (u64, List<Adt1>))> {
        match self {
            Fn4::F35(k0, k1) => f35(*k0, k1.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn5 {
    F37,
}

impl Fn5 {
    pub fn apply(&self, p0: (Adt3, Adt3)) -> (Adt3, Adt3) {
        match self {
            Fn5::F37 => f37(p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn6 {
    F43,
}

impl Fn6 {
    pub fn apply(&self, p0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
        match self {
            Fn6::F43 => f43(p0),
        }
    }
}

pub fn f0(v0: Bytes, v1: Bytes, v2: u64) -> R<Bytes> {
    f1(f2(f3(f4(v0)?, f5()), v1)?, v2)
}

pub fn f1(v0: Adt0, v1: u64) -> R<Bytes> {
    f6(f7(v0)?, v1)
}

pub fn f2(v0: Adt0, v1: Bytes) -> R<Adt0> {
    let v2: Bytes = {
        let a2 = match v0.clone() {
            Adt0::C0(_, _, _, h1, _) => {
                h1
            }
        };
        let a3 = v1;
        append_bytes(a2, a3)
    };
    let v3: u64 = {
        let a4 = v2.clone();
        length_bytes(a4)
    };
    let v4: u64 = {
        let a5 = v3;
        let a6 = 0u64;
        if nat_eq(a5, a6) {
            0u64
        } else {
            let a9 = {
                let a7 = v3;
                let a8 = 1u64;
                nat_sub(a7, a8)
            };
            let a10 = 1024u64;
            let a11 = 0u64;
            nat_quot(a9, a10, a11)
        }
    };
    let v5: u32 = match v0.clone() {
        Adt0::C0(_, h12, _, _, _) => {
            h12
        }
    };
    let v6: Adt1 = match v0.clone() {
        Adt0::C0(h13, _, _, _, _) => {
            h13
        }
    };
    let v7: (u64, List<Adt1>) = f8(Fn0::F9(v5, v2.clone(), v6.clone(), v0.clone()), v4, (0u64, match v0.clone() {
        Adt0::C0(_, _, _, _, h14) => {
            h14
        }
    }))?;
    let v8: u64 = {
        let a15 = v4;
        let a16 = 1024u64;
        nat_mul(a15, a16)?
    };
    Ok(Adt0::C0(v6, v5, {
        let a18 = match v0 {
            Adt0::C0(_, _, h17, _, _) => {
                h17
            }
        };
        let a19 = v4;
        nat_add(a18, a19)?
    }, {
        let m25 = {
            let a22 = v2;
            let a23 = v8;
            let a24 = {
                let a20 = v3;
                let a21 = v8;
                nat_sub(a20, a21)
            };
            slice_bytes(a22, a23, a24)
        };
        match m25 {
            None => {
                Bytes::lit(&[])
            }
            Some(v9) => {
                v9
            }
        }
    }, {
        let (_, h26) = v7;
        h26
    }))
}

pub fn f3(v0: Adt1, v1: u32) -> Adt0 {
    Adt0::C0(v0, v1, 0u64, Bytes::lit(&[]), List::<Adt1>::nil())
}

pub fn f4(v0: Bytes) -> R<Adt1> {
    Ok(f10(f11(f7(f2(f3(f12(), f13()), v0)?)?, 0u64)))
}

pub fn f5() -> u32 {
    64u32
}

pub fn f6(v0: Adt2, v1: u64) -> R<Bytes> {
    let v2: u64 = {
        let a29 = {
            let a27 = v1;
            let a28 = 63u64;
            nat_add(a27, a28)?
        };
        let a30 = 64u64;
        let a31 = 0u64;
        nat_quot(a29, a30, a31)
    };
    let v3: (u64, Bytes) = f14(Fn1::F15(v0), v2, (0u64, Bytes::lit(&[])))?;
    let m36 = {
        let a33 = {
            let (_, h32) = v3.clone();
            h32
        };
        let a34 = 0u64;
        let a35 = v1;
        slice_bytes(a33, a34, a35)
    };
    match m36 {
        None => {
            let (_, h37) = v3;
            Ok(h37)
        }
        Some(v4) => {
            Ok(v4)
        }
    }
}

pub fn f7(v0: Adt0) -> R<Adt2> {
    let v1: Adt1 = match v0.clone() {
        Adt0::C0(h38, _, _, _, _) => {
            h38
        }
    };
    let v2: u32 = match v0.clone() {
        Adt0::C0(_, h39, _, _, _) => {
            h39
        }
    };
    let v3: Bytes = match v0.clone() {
        Adt0::C0(_, _, _, h40, _) => {
            h40
        }
    };
    Ok(f16(Fn2::F17(v2, v1.clone()), f18(v1, v2, v3.clone(), 0u64, {
        let a41 = v3;
        length_bytes(a41)
    }, match v0.clone() {
        Adt0::C0(_, _, h42, _, _) => {
            h42
        }
    })?, match v0 {
        Adt0::C0(_, _, _, _, h43) => {
            h43
        }
    }))
}

pub fn f8(v0: Fn0, p47: u64, p48: (u64, List<Adt1>)) -> R<(u64, List<Adt1>)> {
    let m46 = (p47, p48);
    {
        let mut m46 = m46;
        loop {
            let step = {
                let (v1, v2) = m46;
                let m44 = v1;
                if m44 == 0 {
                    Err::<(u64, (u64, List<Adt1>)), R<(u64, List<Adt1>)>>(Ok(v2))
                } else {
                    let v3 = m44 - 1;
                    Ok::<(u64, (u64, List<Adt1>)), R<(u64, List<Adt1>)>>((v3, {
                        let c45: Fn0 = v0.clone();
                        c45.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m46 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f9(v0: u32, v1: Bytes, v2: Adt1, v3: Adt0, v4: (u64, List<Adt1>)) -> R<(u64, List<Adt1>)> {
    let v5: u64 = {
        let a51 = match v3 {
            Adt0::C0(_, _, h49, _, _) => {
                h49
            }
        };
        let a52 = {
            let (h50, _) = v4.clone();
            h50
        };
        nat_add(a51, a52)?
    };
    Ok(({
        let a54 = {
            let (h53, _) = v4.clone();
            h53
        };
        let a55 = 1u64;
        nat_add(a54, a55)?
    }, f19(v2.clone(), v0, {
        let (_, h56) = v4;
        h56
    }, f20(f18(v2, v0, v1, {
        let a58 = {
            let (h57, _) = v4;
            h57
        };
        let a59 = 1024u64;
        nat_mul(a58, a59)?
    }, 1024u64, v5)?), {
        let a60 = v5;
        let a61 = 1u64;
        nat_add(a60, a61)?
    })))
}

pub fn f10(v0: Adt3) -> Adt1 {
    Adt1::C0(match v0.clone() {
        Adt3::C0(h62, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h62
        }
    }, match v0.clone() {
        Adt3::C0(_, h63, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h63
        }
    }, match v0.clone() {
        Adt3::C0(_, _, h64, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h64
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h65, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h65
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h66, _, _, _, _, _, _, _, _, _, _, _) => {
            h66
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h67, _, _, _, _, _, _, _, _, _, _) => {
            h67
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h68, _, _, _, _, _, _, _, _, _) => {
            h68
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, h69, _, _, _, _, _, _, _, _) => {
            h69
        }
    })
}

pub fn f11(v0: Adt2, v1: u64) -> Adt3 {
    f21(match v0.clone() {
        Adt2::C0(h70, _, _, _, _) => {
            h70
        }
    }, match v0.clone() {
        Adt2::C0(_, h71, _, _, _) => {
            h71
        }
    }, f22(v1), f23(v1), match v0.clone() {
        Adt2::C0(_, _, _, h72, _) => {
            h72
        }
    }, {
        let a74 = match v0 {
            Adt2::C0(_, _, _, _, h73) => {
                h73
            }
        };
        let a75 = f24();
        fixed_u32::bit_or(a74, a75)
    })
}

pub fn f12() -> Adt1 {
    Adt1::C0(1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, 1359893119u32, 2600822924u32, 528734635u32, 1541459225u32)
}

pub fn f13() -> u32 {
    32u32
}

pub fn f14(v0: Fn1, p79: u64, p80: (u64, Bytes)) -> R<(u64, Bytes)> {
    let m78 = (p79, p80);
    {
        let mut m78 = m78;
        loop {
            let step = {
                let (v1, v2) = m78;
                let m76 = v1;
                if m76 == 0 {
                    Err::<(u64, (u64, Bytes)), R<(u64, Bytes)>>(Ok(v2))
                } else {
                    let v3 = m76 - 1;
                    Ok::<(u64, (u64, Bytes)), R<(u64, Bytes)>>((v3, {
                        let c77: Fn1 = v0.clone();
                        c77.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m78 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f15(v0: Adt2, v1: (u64, Bytes)) -> R<(u64, Bytes)> {
    Ok(({
        let a82 = {
            let (h81, _) = v1.clone();
            h81
        };
        let a83 = 1u64;
        nat_add(a82, a83)?
    }, {
        let a86 = {
            let (_, h84) = v1;
            h84
        };
        let a87 = f25(f11(v0, {
            let (h85, _) = v1;
            h85
        }));
        append_bytes(a86, a87)
    }))
}

pub fn f16(v0: Fn2, p91: Adt2, p92: List<Adt1>) -> Adt2 {
    let m90 = (p91, p92);
    {
        let mut m90 = m90;
        loop {
            let step = {
                let (v1, v2) = m90;
                let m88 = v2;
                match m88.uncons() {
                    None => {
                        Err::<(Adt2, List<Adt1>), Adt2>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt2, List<Adt1>), Adt2>(({
                            let c89: Fn2 = v0.clone();
                            c89.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m90 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f17(v0: u32, v1: Adt1, v2: Adt2, v3: Adt1) -> Adt2 {
    f26(v3, f20(v2), v1, v0)
}

pub fn f18(v0: Adt1, v1: u32, v2: Bytes, v3: u64, v4: u64, v5: u64) -> R<Adt2> {
    let v6: u64 = {
        let a93 = v3;
        let a94 = v4;
        nat_add(a93, a94)?
    };
    let v7: u64 = f27(v4)?;
    let v8: u32 = f22(v5);
    let v9: u32 = f23(v5);
    let v10: (u64, Adt1) = f28(Fn3::F29(v1, v3, v9, v6, v8, v2.clone()), {
        let a95 = v7;
        let a96 = 64u64;
        let a97 = 0u64;
        nat_quot(a95, a96, a97)
    }, (0u64, v0))?;
    Ok(Adt2::C0({
        let (_, h98) = v10;
        h98
    }, f30(v2, v6, {
        let a99 = v3;
        let a100 = v7;
        nat_add(a99, a100)?
    })?, v5, f31({
        let a101 = v4;
        let a102 = v7;
        nat_sub(a101, a102)
    }, 7u64), {
        let a105 = v1;
        let a106 = {
            let a103 = f32();
            let a104 = f33(v7);
            fixed_u32::bit_or(a103, a104)
        };
        fixed_u32::bit_or(a105, a106)
    }))
}

pub fn f19(v0: Adt1, v1: u32, v2: List<Adt1>, v3: Adt1, v4: u64) -> List<Adt1> {
    let v5: (Adt1, (u64, List<Adt1>)) = {
        let (h107, _) = f34(Fn4::F35(v1, v0), 64u64, (v3, (v4, v2)));
        h107
    };
    List::cons({
        let (h108, _) = v5;
        h108
    }, {
        let (_, h109) = {
            let (_, h110) = v5;
            h110
        };
        h109
    })
}

pub fn f20(v0: Adt2) -> Adt1 {
    f10(f21(match v0.clone() {
        Adt2::C0(h111, _, _, _, _) => {
            h111
        }
    }, match v0.clone() {
        Adt2::C0(_, h112, _, _, _) => {
            h112
        }
    }, f22(match v0.clone() {
        Adt2::C0(_, _, h113, _, _) => {
            h113
        }
    }), f23(match v0.clone() {
        Adt2::C0(_, _, h114, _, _) => {
            h114
        }
    }), match v0.clone() {
        Adt2::C0(_, _, _, h115, _) => {
            h115
        }
    }, match v0 {
        Adt2::C0(_, _, _, _, h116) => {
            h116
        }
    }))
}

pub fn f21(v0: Adt1, v1: Adt3, v2: u32, v3: u32, v4: u32, v5: u32) -> Adt3 {
    let v6: Adt3 = {
        let (h117, _) = f36(Fn5::F37, 7u64, (Adt3::C0(match v0.clone() {
            Adt1::C0(h118, _, _, _, _, _, _, _) => {
                h118
            }
        }, match v0.clone() {
            Adt1::C0(_, h119, _, _, _, _, _, _) => {
                h119
            }
        }, match v0.clone() {
            Adt1::C0(_, _, h120, _, _, _, _, _) => {
                h120
            }
        }, match v0.clone() {
            Adt1::C0(_, _, _, h121, _, _, _, _) => {
                h121
            }
        }, match v0.clone() {
            Adt1::C0(_, _, _, _, h122, _, _, _) => {
                h122
            }
        }, match v0.clone() {
            Adt1::C0(_, _, _, _, _, h123, _, _) => {
                h123
            }
        }, match v0.clone() {
            Adt1::C0(_, _, _, _, _, _, h124, _) => {
                h124
            }
        }, match v0.clone() {
            Adt1::C0(_, _, _, _, _, _, _, h125) => {
                h125
            }
        }, 1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, v2, v3, v4, v5), v1));
        h117
    };
    Adt3::C0(f38(match v6.clone() {
        Adt3::C0(h126, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h126
        }
    }, match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, h127, _, _, _, _, _, _, _) => {
            h127
        }
    }), f38(match v6.clone() {
        Adt3::C0(_, h128, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h128
        }
    }, match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h129, _, _, _, _, _, _) => {
            h129
        }
    }), f38(match v6.clone() {
        Adt3::C0(_, _, h130, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h130
        }
    }, match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h131, _, _, _, _, _) => {
            h131
        }
    }), f38(match v6.clone() {
        Adt3::C0(_, _, _, h132, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h132
        }
    }, match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h133, _, _, _, _) => {
            h133
        }
    }), f38(match v6.clone() {
        Adt3::C0(_, _, _, _, h134, _, _, _, _, _, _, _, _, _, _, _) => {
            h134
        }
    }, match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h135, _, _, _) => {
            h135
        }
    }), f38(match v6.clone() {
        Adt3::C0(_, _, _, _, _, h136, _, _, _, _, _, _, _, _, _, _) => {
            h136
        }
    }, match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h137, _, _) => {
            h137
        }
    }), f38(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, h138, _, _, _, _, _, _, _, _, _) => {
            h138
        }
    }, match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h139, _) => {
            h139
        }
    }), f38(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, h140, _, _, _, _, _, _, _, _) => {
            h140
        }
    }, match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h141) => {
            h141
        }
    }), f38(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, h142, _, _, _, _, _, _, _) => {
            h142
        }
    }, match v0.clone() {
        Adt1::C0(h143, _, _, _, _, _, _, _) => {
            h143
        }
    }), f38(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h144, _, _, _, _, _, _) => {
            h144
        }
    }, match v0.clone() {
        Adt1::C0(_, h145, _, _, _, _, _, _) => {
            h145
        }
    }), f38(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h146, _, _, _, _, _) => {
            h146
        }
    }, match v0.clone() {
        Adt1::C0(_, _, h147, _, _, _, _, _) => {
            h147
        }
    }), f38(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h148, _, _, _, _) => {
            h148
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, h149, _, _, _, _) => {
            h149
        }
    }), f38(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h150, _, _, _) => {
            h150
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, h151, _, _, _) => {
            h151
        }
    }), f38(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h152, _, _) => {
            h152
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, _, h153, _, _) => {
            h153
        }
    }), f38(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h154, _) => {
            h154
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, _, _, h155, _) => {
            h155
        }
    }), f38(match v6 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h156) => {
            h156
        }
    }, match v0 {
        Adt1::C0(_, _, _, _, _, _, _, h157) => {
            h157
        }
    }))
}

pub fn f22(v0: u64) -> u32 {
    f31({
        let a158 = v0;
        let a159 = 4294967296u64;
        let a160 = 0u64;
        nat_rem(a158, a159, a160)
    }, 32u64)
}

pub fn f23(v0: u64) -> u32 {
    let a161 = v0;
    let a162 = 4294967296u64;
    if nat_lt(a161, a162) {
        0u32
    } else {
        f31({
            let a163 = v0;
            let a164 = 4294967296u64;
            let a165 = 0u64;
            nat_quot(a163, a164, a165)
        }, 32u64)
    }
}

pub fn f24() -> u32 {
    8u32
}

pub fn f25(v0: Adt3) -> Bytes {
    let a210 = f39(match v0.clone() {
        Adt3::C0(h166, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h166
        }
    });
    let a211 = {
        let a208 = f39(match v0.clone() {
            Adt3::C0(_, h167, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                h167
            }
        });
        let a209 = {
            let a206 = f39(match v0.clone() {
                Adt3::C0(_, _, h168, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                    h168
                }
            });
            let a207 = {
                let a204 = f39(match v0.clone() {
                    Adt3::C0(_, _, _, h169, _, _, _, _, _, _, _, _, _, _, _, _) => {
                        h169
                    }
                });
                let a205 = {
                    let a202 = f39(match v0.clone() {
                        Adt3::C0(_, _, _, _, h170, _, _, _, _, _, _, _, _, _, _, _) => {
                            h170
                        }
                    });
                    let a203 = {
                        let a200 = f39(match v0.clone() {
                            Adt3::C0(_, _, _, _, _, h171, _, _, _, _, _, _, _, _, _, _) => {
                                h171
                            }
                        });
                        let a201 = {
                            let a198 = f39(match v0.clone() {
                                Adt3::C0(_, _, _, _, _, _, h172, _, _, _, _, _, _, _, _, _) => {
                                    h172
                                }
                            });
                            let a199 = {
                                let a196 = f39(match v0.clone() {
                                    Adt3::C0(_, _, _, _, _, _, _, h173, _, _, _, _, _, _, _, _) => {
                                        h173
                                    }
                                });
                                let a197 = {
                                    let a194 = f39(match v0.clone() {
                                        Adt3::C0(_, _, _, _, _, _, _, _, h174, _, _, _, _, _, _, _) => {
                                            h174
                                        }
                                    });
                                    let a195 = {
                                        let a192 = f39(match v0.clone() {
                                            Adt3::C0(_, _, _, _, _, _, _, _, _, h175, _, _, _, _, _, _) => {
                                                h175
                                            }
                                        });
                                        let a193 = {
                                            let a190 = f39(match v0.clone() {
                                                Adt3::C0(_, _, _, _, _, _, _, _, _, _, h176, _, _, _, _, _) => {
                                                    h176
                                                }
                                            });
                                            let a191 = {
                                                let a188 = f39(match v0.clone() {
                                                    Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h177, _, _, _, _) => {
                                                        h177
                                                    }
                                                });
                                                let a189 = {
                                                    let a186 = f39(match v0.clone() {
                                                        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h178, _, _, _) => {
                                                            h178
                                                        }
                                                    });
                                                    let a187 = {
                                                        let a184 = f39(match v0.clone() {
                                                            Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h179, _, _) => {
                                                                h179
                                                            }
                                                        });
                                                        let a185 = {
                                                            let a182 = f39(match v0.clone() {
                                                                Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h180, _) => {
                                                                    h180
                                                                }
                                                            });
                                                            let a183 = f39(match v0 {
                                                                Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h181) => {
                                                                    h181
                                                                }
                                                            });
                                                            append_bytes(a182, a183)
                                                        };
                                                        append_bytes(a184, a185)
                                                    };
                                                    append_bytes(a186, a187)
                                                };
                                                append_bytes(a188, a189)
                                            };
                                            append_bytes(a190, a191)
                                        };
                                        append_bytes(a192, a193)
                                    };
                                    append_bytes(a194, a195)
                                };
                                append_bytes(a196, a197)
                            };
                            append_bytes(a198, a199)
                        };
                        append_bytes(a200, a201)
                    };
                    append_bytes(a202, a203)
                };
                append_bytes(a204, a205)
            };
            append_bytes(a206, a207)
        };
        append_bytes(a208, a209)
    };
    append_bytes(a210, a211)
}

pub fn f26(v0: Adt1, v1: Adt1, v2: Adt1, v3: u32) -> Adt2 {
    Adt2::C0(v2, Adt3::C0(match v0.clone() {
        Adt1::C0(h212, _, _, _, _, _, _, _) => {
            h212
        }
    }, match v0.clone() {
        Adt1::C0(_, h213, _, _, _, _, _, _) => {
            h213
        }
    }, match v0.clone() {
        Adt1::C0(_, _, h214, _, _, _, _, _) => {
            h214
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, h215, _, _, _, _) => {
            h215
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, h216, _, _, _) => {
            h216
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, _, h217, _, _) => {
            h217
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, _, _, h218, _) => {
            h218
        }
    }, match v0 {
        Adt1::C0(_, _, _, _, _, _, _, h219) => {
            h219
        }
    }, match v1.clone() {
        Adt1::C0(h220, _, _, _, _, _, _, _) => {
            h220
        }
    }, match v1.clone() {
        Adt1::C0(_, h221, _, _, _, _, _, _) => {
            h221
        }
    }, match v1.clone() {
        Adt1::C0(_, _, h222, _, _, _, _, _) => {
            h222
        }
    }, match v1.clone() {
        Adt1::C0(_, _, _, h223, _, _, _, _) => {
            h223
        }
    }, match v1.clone() {
        Adt1::C0(_, _, _, _, h224, _, _, _) => {
            h224
        }
    }, match v1.clone() {
        Adt1::C0(_, _, _, _, _, h225, _, _) => {
            h225
        }
    }, match v1.clone() {
        Adt1::C0(_, _, _, _, _, _, h226, _) => {
            h226
        }
    }, match v1 {
        Adt1::C0(_, _, _, _, _, _, _, h227) => {
            h227
        }
    }), 0u64, 64u32, {
        let a228 = v3;
        let a229 = f40();
        fixed_u32::bit_or(a228, a229)
    })
}

pub fn f27(v0: u64) -> R<u64> {
    let a230 = v0;
    let a231 = 0u64;
    if nat_eq(a230, a231) {
        Ok(0u64)
    } else {
        let a237 = {
            let a234 = {
                let a232 = v0;
                let a233 = 1u64;
                nat_sub(a232, a233)
            };
            let a235 = 64u64;
            let a236 = 0u64;
            nat_quot(a234, a235, a236)
        };
        let a238 = 64u64;
        nat_mul(a237, a238)
    }
}

pub fn f28(v0: Fn3, p242: u64, p243: (u64, Adt1)) -> R<(u64, Adt1)> {
    let m241 = (p242, p243);
    {
        let mut m241 = m241;
        loop {
            let step = {
                let (v1, v2) = m241;
                let m239 = v1;
                if m239 == 0 {
                    Err::<(u64, (u64, Adt1)), R<(u64, Adt1)>>(Ok(v2))
                } else {
                    let v3 = m239 - 1;
                    Ok::<(u64, (u64, Adt1)), R<(u64, Adt1)>>((v3, {
                        let c240: Fn3 = v0.clone();
                        c240.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m241 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f29(v0: u32, v1: u64, v2: u32, v3: u64, v4: u32, v5: Bytes, v6: (u64, Adt1)) -> R<(u64, Adt1)> {
    let v7: u64 = {
        let (h244, _) = v6;
        h244
    };
    Ok(({
        let a245 = v7;
        let a246 = 64u64;
        nat_add(a245, a246)?
    }, f10(f21({
        let (_, h247) = v6;
        h247
    }, f30(v5, v3, {
        let a248 = v1;
        let a249 = v7;
        nat_add(a248, a249)?
    })?, v4, v2, 64u32, {
        let a250 = v0;
        let a251 = f33(v7);
        fixed_u32::bit_or(a250, a251)
    }))))
}

pub fn f30(v0: Bytes, v1: u64, v2: u64) -> R<Adt3> {
    Ok(Adt3::C0(f41(v0.clone(), v1, {
        let a252 = v2;
        let a253 = 0u64;
        nat_add(a252, a253)?
    })?, f41(v0.clone(), v1, {
        let a254 = v2;
        let a255 = 4u64;
        nat_add(a254, a255)?
    })?, f41(v0.clone(), v1, {
        let a256 = v2;
        let a257 = 8u64;
        nat_add(a256, a257)?
    })?, f41(v0.clone(), v1, {
        let a258 = v2;
        let a259 = 12u64;
        nat_add(a258, a259)?
    })?, f41(v0.clone(), v1, {
        let a260 = v2;
        let a261 = 16u64;
        nat_add(a260, a261)?
    })?, f41(v0.clone(), v1, {
        let a262 = v2;
        let a263 = 20u64;
        nat_add(a262, a263)?
    })?, f41(v0.clone(), v1, {
        let a264 = v2;
        let a265 = 24u64;
        nat_add(a264, a265)?
    })?, f41(v0.clone(), v1, {
        let a266 = v2;
        let a267 = 28u64;
        nat_add(a266, a267)?
    })?, f41(v0.clone(), v1, {
        let a268 = v2;
        let a269 = 32u64;
        nat_add(a268, a269)?
    })?, f41(v0.clone(), v1, {
        let a270 = v2;
        let a271 = 36u64;
        nat_add(a270, a271)?
    })?, f41(v0.clone(), v1, {
        let a272 = v2;
        let a273 = 40u64;
        nat_add(a272, a273)?
    })?, f41(v0.clone(), v1, {
        let a274 = v2;
        let a275 = 44u64;
        nat_add(a274, a275)?
    })?, f41(v0.clone(), v1, {
        let a276 = v2;
        let a277 = 48u64;
        nat_add(a276, a277)?
    })?, f41(v0.clone(), v1, {
        let a278 = v2;
        let a279 = 52u64;
        nat_add(a278, a279)?
    })?, f41(v0.clone(), v1, {
        let a280 = v2;
        let a281 = 56u64;
        nat_add(a280, a281)?
    })?, f41(v0, v1, {
        let a282 = v2;
        let a283 = 60u64;
        nat_add(a282, a283)?
    })?))
}

pub fn f31(v0: u64, v1: u64) -> u32 {
    let (_, h284) = {
        let (_, h285) = f42(Fn6::F43, v1, (v0, (1u32, 0u32)));
        h285
    };
    h284
}

pub fn f32() -> u32 {
    2u32
}

pub fn f33(v0: u64) -> u32 {
    let a286 = v0;
    let a287 = 0u64;
    if nat_eq(a286, a287) {
        f44()
    } else {
        0u32
    }
}

pub fn f34(v0: Fn4, p292: u64, p293: (Adt1, (u64, List<Adt1>))) -> ((Adt1, (u64, List<Adt1>)), bool) {
    let m291 = (p292, p293);
    {
        let mut m291 = m291;
        loop {
            let step = {
                let (v1, v2) = m291;
                let m288 = v1;
                if m288 == 0 {
                    Err::<(u64, (Adt1, (u64, List<Adt1>))), ((Adt1, (u64, List<Adt1>)), bool)>((v2, false))
                } else {
                    let v3 = m288 - 1;
                    let m290 = {
                        let c289: Fn4 = v0.clone();
                        c289.apply(v2.clone())
                    };
                    match m290 {
                        None => {
                            Err::<(u64, (Adt1, (u64, List<Adt1>))), ((Adt1, (u64, List<Adt1>)), bool)>((v2, true))
                        }
                        Some(v4) => {
                            Ok::<(u64, (Adt1, (u64, List<Adt1>))), ((Adt1, (u64, List<Adt1>)), bool)>((v3, v4))
                        }
                    }
                }
            };
            match step {
                Ok(next) => { m291 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f35(v0: u32, v1: Adt1, v2: (Adt1, (u64, List<Adt1>))) -> Option<(Adt1, (u64, List<Adt1>))> {
    f45(v1, v0, v2)
}

pub fn f36(v0: Fn5, p297: u64, p298: (Adt3, Adt3)) -> (Adt3, Adt3) {
    let m296 = (p297, p298);
    {
        let mut m296 = m296;
        loop {
            let step = {
                let (v1, v2) = m296;
                let m294 = v1;
                if m294 == 0 {
                    Err::<(u64, (Adt3, Adt3)), (Adt3, Adt3)>(v2)
                } else {
                    let v3 = m294 - 1;
                    Ok::<(u64, (Adt3, Adt3)), (Adt3, Adt3)>((v3, {
                        let c295: Fn5 = v0.clone();
                        c295.apply(v2)
                    }))
                }
            };
            match step {
                Ok(next) => { m296 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f37(v0: (Adt3, Adt3)) -> (Adt3, Adt3) {
    (f46({
        let (h299, _) = v0.clone();
        h299
    }, {
        let (_, h300) = v0.clone();
        h300
    }), f47({
        let (_, h301) = v0;
        h301
    }))
}

pub fn f38(v0: u32, v1: u32) -> u32 {
    let a302 = v0;
    let a303 = v1;
    fixed_u32::bit_xor(a302, a303)
}

pub fn f39(v0: u32) -> Bytes {
    let a314 = f48({
        let a304 = v0;
        let a305 = 255u32;
        fixed_u32::bit_and(a304, a305)
    });
    let a315 = {
        let a312 = f48({
            let a306 = f49(v0, 8u32);
            let a307 = 255u32;
            fixed_u32::bit_and(a306, a307)
        });
        let a313 = {
            let a310 = f48({
                let a308 = f49(v0, 16u32);
                let a309 = 255u32;
                fixed_u32::bit_and(a308, a309)
            });
            let a311 = f48(f49(v0, 24u32));
            append_bytes(a310, a311)
        };
        append_bytes(a312, a313)
    };
    append_bytes(a314, a315)
}

pub fn f40() -> u32 {
    4u32
}

pub fn f41(v0: Bytes, v1: u64, v2: u64) -> R<u32> {
    Ok(f50(f51(v0.clone(), v1, v2), f51(v0.clone(), v1, {
        let a316 = v2;
        let a317 = 1u64;
        nat_add(a316, a317)?
    }), f51(v0.clone(), v1, {
        let a318 = v2;
        let a319 = 2u64;
        nat_add(a318, a319)?
    }), f51(v0, v1, {
        let a320 = v2;
        let a321 = 3u64;
        nat_add(a320, a321)?
    })))
}

pub fn f42(v0: Fn6, p325: u64, p326: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let m324 = (p325, p326);
    {
        let mut m324 = m324;
        loop {
            let step = {
                let (v1, v2) = m324;
                let m322 = v1;
                if m322 == 0 {
                    Err::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>(v2)
                } else {
                    let v3 = m322 - 1;
                    Ok::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>((v3, {
                        let c323: Fn6 = v0.clone();
                        c323.apply(v2)
                    }))
                }
            };
            match step {
                Ok(next) => { m324 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f43(v0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let v1: u64 = {
        let (h327, _) = v0;
        h327
    };
    let v2: u32 = {
        let (h328, _) = {
            let (_, h329) = v0;
            h329
        };
        h328
    };
    let v3: u32 = {
        let (_, h330) = {
            let (_, h331) = v0;
            h331
        };
        h330
    };
    ({
        let a332 = v1;
        let a333 = 2u64;
        let a334 = 0u64;
        nat_quot(a332, a333, a334)
    }, (f52(v2, 1u32), {
        let a338 = {
            let a335 = v1;
            let a336 = 2u64;
            let a337 = 0u64;
            nat_rem(a335, a336, a337)
        };
        let a339 = 1u64;
        if nat_eq(a338, a339) {
            let a340 = v3;
            let a341 = v2;
            fixed_u32::bit_or(a340, a341)
        } else {
            v3
        }
    }))
}

pub fn f44() -> u32 {
    1u32
}

pub fn f45(v0: Adt1, v1: u32, v2: (Adt1, (u64, List<Adt1>))) -> Option<(Adt1, (u64, List<Adt1>))> {
    let v3: u64 = {
        let (h342, _) = {
            let (_, h343) = v2.clone();
            h343
        };
        h342
    };
    let a347 = {
        let a344 = v3;
        let a345 = 2u64;
        let a346 = 0u64;
        nat_rem(a344, a345, a346)
    };
    let a348 = 0u64;
    if nat_eq(a347, a348) {
        let m351 = {
            let (_, h349) = {
                let (_, h350) = v2;
                h350
            };
            h349
        };
        match m351.uncons() {
            None => {
                None::<(Adt1, (u64, List<Adt1>))>
            }
            Some((v4, v5)) => {
                Some((f20(f26(v4, {
                    let (h352, _) = v2;
                    h352
                }, v0, v1)), ({
                    let a353 = v3;
                    let a354 = 2u64;
                    let a355 = 0u64;
                    nat_quot(a353, a354, a355)
                }, v5)))
            }
        }
    } else {
        None::<(Adt1, (u64, List<Adt1>))>
    }
}

pub fn f46(v0: Adt3, v1: Adt3) -> Adt3 {
    f53(f54(f55(f56(f57(f58(f59(f60(v0, v1.clone()), v1.clone()), v1.clone()), v1.clone()), v1.clone()), v1.clone()), v1.clone()), v1)
}

pub fn f47(v0: Adt3) -> Adt3 {
    Adt3::C0(match v0.clone() {
        Adt3::C0(_, _, h356, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h356
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h357, _, _, _, _, _, _, _, _, _) => {
            h357
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h358, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h358
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h359, _, _, _, _, _) => {
            h359
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, h360, _, _, _, _, _, _, _, _) => {
            h360
        }
    }, match v0.clone() {
        Adt3::C0(h361, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h361
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h362, _, _, _, _, _, _, _, _, _, _, _) => {
            h362
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h363, _, _) => {
            h363
        }
    }, match v0.clone() {
        Adt3::C0(_, h364, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h364
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h365, _, _, _, _) => {
            h365
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h366, _, _, _) => {
            h366
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h367, _, _, _, _, _, _, _, _, _, _) => {
            h367
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h368, _, _, _, _, _, _) => {
            h368
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h369, _) => {
            h369
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h370) => {
            h370
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, _, h371, _, _, _, _, _, _, _) => {
            h371
        }
    })
}

pub fn f48(v0: u32) -> Bytes {
    let a374 = {
        let a372 = v0;
        let a373 = 128u32;
        fixed_u32::bit_and(a372, a373)
    };
    let a375 = 0u32;
    if equal(a374, a375) {
        let a378 = {
            let a376 = v0;
            let a377 = 64u32;
            fixed_u32::bit_and(a376, a377)
        };
        let a379 = 0u32;
        if equal(a378, a379) {
            let a382 = {
                let a380 = v0;
                let a381 = 32u32;
                fixed_u32::bit_and(a380, a381)
            };
            let a383 = 0u32;
            if equal(a382, a383) {
                let a386 = {
                    let a384 = v0;
                    let a385 = 16u32;
                    fixed_u32::bit_and(a384, a385)
                };
                let a387 = 0u32;
                if equal(a386, a387) {
                    let a390 = {
                        let a388 = v0;
                        let a389 = 8u32;
                        fixed_u32::bit_and(a388, a389)
                    };
                    let a391 = 0u32;
                    if equal(a390, a391) {
                        let a394 = {
                            let a392 = v0;
                            let a393 = 4u32;
                            fixed_u32::bit_and(a392, a393)
                        };
                        let a395 = 0u32;
                        if equal(a394, a395) {
                            let a398 = {
                                let a396 = v0;
                                let a397 = 2u32;
                                fixed_u32::bit_and(a396, a397)
                            };
                            let a399 = 0u32;
                            if equal(a398, a399) {
                                let a402 = {
                                    let a400 = v0;
                                    let a401 = 1u32;
                                    fixed_u32::bit_and(a400, a401)
                                };
                                let a403 = 0u32;
                                if equal(a402, a403) {
                                    Bytes::lit(&[0x00])
                                } else {
                                    Bytes::lit(&[0x01])
                                }
                            } else {
                                let a406 = {
                                    let a404 = v0;
                                    let a405 = 1u32;
                                    fixed_u32::bit_and(a404, a405)
                                };
                                let a407 = 0u32;
                                if equal(a406, a407) {
                                    Bytes::lit(&[0x02])
                                } else {
                                    Bytes::lit(&[0x03])
                                }
                            }
                        } else {
                            let a410 = {
                                let a408 = v0;
                                let a409 = 2u32;
                                fixed_u32::bit_and(a408, a409)
                            };
                            let a411 = 0u32;
                            if equal(a410, a411) {
                                let a414 = {
                                    let a412 = v0;
                                    let a413 = 1u32;
                                    fixed_u32::bit_and(a412, a413)
                                };
                                let a415 = 0u32;
                                if equal(a414, a415) {
                                    Bytes::lit(&[0x04])
                                } else {
                                    Bytes::lit(&[0x05])
                                }
                            } else {
                                let a418 = {
                                    let a416 = v0;
                                    let a417 = 1u32;
                                    fixed_u32::bit_and(a416, a417)
                                };
                                let a419 = 0u32;
                                if equal(a418, a419) {
                                    Bytes::lit(&[0x06])
                                } else {
                                    Bytes::lit(&[0x07])
                                }
                            }
                        }
                    } else {
                        let a422 = {
                            let a420 = v0;
                            let a421 = 4u32;
                            fixed_u32::bit_and(a420, a421)
                        };
                        let a423 = 0u32;
                        if equal(a422, a423) {
                            let a426 = {
                                let a424 = v0;
                                let a425 = 2u32;
                                fixed_u32::bit_and(a424, a425)
                            };
                            let a427 = 0u32;
                            if equal(a426, a427) {
                                let a430 = {
                                    let a428 = v0;
                                    let a429 = 1u32;
                                    fixed_u32::bit_and(a428, a429)
                                };
                                let a431 = 0u32;
                                if equal(a430, a431) {
                                    Bytes::lit(&[0x08])
                                } else {
                                    Bytes::lit(&[0x09])
                                }
                            } else {
                                let a434 = {
                                    let a432 = v0;
                                    let a433 = 1u32;
                                    fixed_u32::bit_and(a432, a433)
                                };
                                let a435 = 0u32;
                                if equal(a434, a435) {
                                    Bytes::lit(&[0x0a])
                                } else {
                                    Bytes::lit(&[0x0b])
                                }
                            }
                        } else {
                            let a438 = {
                                let a436 = v0;
                                let a437 = 2u32;
                                fixed_u32::bit_and(a436, a437)
                            };
                            let a439 = 0u32;
                            if equal(a438, a439) {
                                let a442 = {
                                    let a440 = v0;
                                    let a441 = 1u32;
                                    fixed_u32::bit_and(a440, a441)
                                };
                                let a443 = 0u32;
                                if equal(a442, a443) {
                                    Bytes::lit(&[0x0c])
                                } else {
                                    Bytes::lit(&[0x0d])
                                }
                            } else {
                                let a446 = {
                                    let a444 = v0;
                                    let a445 = 1u32;
                                    fixed_u32::bit_and(a444, a445)
                                };
                                let a447 = 0u32;
                                if equal(a446, a447) {
                                    Bytes::lit(&[0x0e])
                                } else {
                                    Bytes::lit(&[0x0f])
                                }
                            }
                        }
                    }
                } else {
                    let a450 = {
                        let a448 = v0;
                        let a449 = 8u32;
                        fixed_u32::bit_and(a448, a449)
                    };
                    let a451 = 0u32;
                    if equal(a450, a451) {
                        let a454 = {
                            let a452 = v0;
                            let a453 = 4u32;
                            fixed_u32::bit_and(a452, a453)
                        };
                        let a455 = 0u32;
                        if equal(a454, a455) {
                            let a458 = {
                                let a456 = v0;
                                let a457 = 2u32;
                                fixed_u32::bit_and(a456, a457)
                            };
                            let a459 = 0u32;
                            if equal(a458, a459) {
                                let a462 = {
                                    let a460 = v0;
                                    let a461 = 1u32;
                                    fixed_u32::bit_and(a460, a461)
                                };
                                let a463 = 0u32;
                                if equal(a462, a463) {
                                    Bytes::lit(&[0x10])
                                } else {
                                    Bytes::lit(&[0x11])
                                }
                            } else {
                                let a466 = {
                                    let a464 = v0;
                                    let a465 = 1u32;
                                    fixed_u32::bit_and(a464, a465)
                                };
                                let a467 = 0u32;
                                if equal(a466, a467) {
                                    Bytes::lit(&[0x12])
                                } else {
                                    Bytes::lit(&[0x13])
                                }
                            }
                        } else {
                            let a470 = {
                                let a468 = v0;
                                let a469 = 2u32;
                                fixed_u32::bit_and(a468, a469)
                            };
                            let a471 = 0u32;
                            if equal(a470, a471) {
                                let a474 = {
                                    let a472 = v0;
                                    let a473 = 1u32;
                                    fixed_u32::bit_and(a472, a473)
                                };
                                let a475 = 0u32;
                                if equal(a474, a475) {
                                    Bytes::lit(&[0x14])
                                } else {
                                    Bytes::lit(&[0x15])
                                }
                            } else {
                                let a478 = {
                                    let a476 = v0;
                                    let a477 = 1u32;
                                    fixed_u32::bit_and(a476, a477)
                                };
                                let a479 = 0u32;
                                if equal(a478, a479) {
                                    Bytes::lit(&[0x16])
                                } else {
                                    Bytes::lit(&[0x17])
                                }
                            }
                        }
                    } else {
                        let a482 = {
                            let a480 = v0;
                            let a481 = 4u32;
                            fixed_u32::bit_and(a480, a481)
                        };
                        let a483 = 0u32;
                        if equal(a482, a483) {
                            let a486 = {
                                let a484 = v0;
                                let a485 = 2u32;
                                fixed_u32::bit_and(a484, a485)
                            };
                            let a487 = 0u32;
                            if equal(a486, a487) {
                                let a490 = {
                                    let a488 = v0;
                                    let a489 = 1u32;
                                    fixed_u32::bit_and(a488, a489)
                                };
                                let a491 = 0u32;
                                if equal(a490, a491) {
                                    Bytes::lit(&[0x18])
                                } else {
                                    Bytes::lit(&[0x19])
                                }
                            } else {
                                let a494 = {
                                    let a492 = v0;
                                    let a493 = 1u32;
                                    fixed_u32::bit_and(a492, a493)
                                };
                                let a495 = 0u32;
                                if equal(a494, a495) {
                                    Bytes::lit(&[0x1a])
                                } else {
                                    Bytes::lit(&[0x1b])
                                }
                            }
                        } else {
                            let a498 = {
                                let a496 = v0;
                                let a497 = 2u32;
                                fixed_u32::bit_and(a496, a497)
                            };
                            let a499 = 0u32;
                            if equal(a498, a499) {
                                let a502 = {
                                    let a500 = v0;
                                    let a501 = 1u32;
                                    fixed_u32::bit_and(a500, a501)
                                };
                                let a503 = 0u32;
                                if equal(a502, a503) {
                                    Bytes::lit(&[0x1c])
                                } else {
                                    Bytes::lit(&[0x1d])
                                }
                            } else {
                                let a506 = {
                                    let a504 = v0;
                                    let a505 = 1u32;
                                    fixed_u32::bit_and(a504, a505)
                                };
                                let a507 = 0u32;
                                if equal(a506, a507) {
                                    Bytes::lit(&[0x1e])
                                } else {
                                    Bytes::lit(&[0x1f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a510 = {
                    let a508 = v0;
                    let a509 = 16u32;
                    fixed_u32::bit_and(a508, a509)
                };
                let a511 = 0u32;
                if equal(a510, a511) {
                    let a514 = {
                        let a512 = v0;
                        let a513 = 8u32;
                        fixed_u32::bit_and(a512, a513)
                    };
                    let a515 = 0u32;
                    if equal(a514, a515) {
                        let a518 = {
                            let a516 = v0;
                            let a517 = 4u32;
                            fixed_u32::bit_and(a516, a517)
                        };
                        let a519 = 0u32;
                        if equal(a518, a519) {
                            let a522 = {
                                let a520 = v0;
                                let a521 = 2u32;
                                fixed_u32::bit_and(a520, a521)
                            };
                            let a523 = 0u32;
                            if equal(a522, a523) {
                                let a526 = {
                                    let a524 = v0;
                                    let a525 = 1u32;
                                    fixed_u32::bit_and(a524, a525)
                                };
                                let a527 = 0u32;
                                if equal(a526, a527) {
                                    Bytes::lit(&[0x20])
                                } else {
                                    Bytes::lit(&[0x21])
                                }
                            } else {
                                let a530 = {
                                    let a528 = v0;
                                    let a529 = 1u32;
                                    fixed_u32::bit_and(a528, a529)
                                };
                                let a531 = 0u32;
                                if equal(a530, a531) {
                                    Bytes::lit(&[0x22])
                                } else {
                                    Bytes::lit(&[0x23])
                                }
                            }
                        } else {
                            let a534 = {
                                let a532 = v0;
                                let a533 = 2u32;
                                fixed_u32::bit_and(a532, a533)
                            };
                            let a535 = 0u32;
                            if equal(a534, a535) {
                                let a538 = {
                                    let a536 = v0;
                                    let a537 = 1u32;
                                    fixed_u32::bit_and(a536, a537)
                                };
                                let a539 = 0u32;
                                if equal(a538, a539) {
                                    Bytes::lit(&[0x24])
                                } else {
                                    Bytes::lit(&[0x25])
                                }
                            } else {
                                let a542 = {
                                    let a540 = v0;
                                    let a541 = 1u32;
                                    fixed_u32::bit_and(a540, a541)
                                };
                                let a543 = 0u32;
                                if equal(a542, a543) {
                                    Bytes::lit(&[0x26])
                                } else {
                                    Bytes::lit(&[0x27])
                                }
                            }
                        }
                    } else {
                        let a546 = {
                            let a544 = v0;
                            let a545 = 4u32;
                            fixed_u32::bit_and(a544, a545)
                        };
                        let a547 = 0u32;
                        if equal(a546, a547) {
                            let a550 = {
                                let a548 = v0;
                                let a549 = 2u32;
                                fixed_u32::bit_and(a548, a549)
                            };
                            let a551 = 0u32;
                            if equal(a550, a551) {
                                let a554 = {
                                    let a552 = v0;
                                    let a553 = 1u32;
                                    fixed_u32::bit_and(a552, a553)
                                };
                                let a555 = 0u32;
                                if equal(a554, a555) {
                                    Bytes::lit(&[0x28])
                                } else {
                                    Bytes::lit(&[0x29])
                                }
                            } else {
                                let a558 = {
                                    let a556 = v0;
                                    let a557 = 1u32;
                                    fixed_u32::bit_and(a556, a557)
                                };
                                let a559 = 0u32;
                                if equal(a558, a559) {
                                    Bytes::lit(&[0x2a])
                                } else {
                                    Bytes::lit(&[0x2b])
                                }
                            }
                        } else {
                            let a562 = {
                                let a560 = v0;
                                let a561 = 2u32;
                                fixed_u32::bit_and(a560, a561)
                            };
                            let a563 = 0u32;
                            if equal(a562, a563) {
                                let a566 = {
                                    let a564 = v0;
                                    let a565 = 1u32;
                                    fixed_u32::bit_and(a564, a565)
                                };
                                let a567 = 0u32;
                                if equal(a566, a567) {
                                    Bytes::lit(&[0x2c])
                                } else {
                                    Bytes::lit(&[0x2d])
                                }
                            } else {
                                let a570 = {
                                    let a568 = v0;
                                    let a569 = 1u32;
                                    fixed_u32::bit_and(a568, a569)
                                };
                                let a571 = 0u32;
                                if equal(a570, a571) {
                                    Bytes::lit(&[0x2e])
                                } else {
                                    Bytes::lit(&[0x2f])
                                }
                            }
                        }
                    }
                } else {
                    let a574 = {
                        let a572 = v0;
                        let a573 = 8u32;
                        fixed_u32::bit_and(a572, a573)
                    };
                    let a575 = 0u32;
                    if equal(a574, a575) {
                        let a578 = {
                            let a576 = v0;
                            let a577 = 4u32;
                            fixed_u32::bit_and(a576, a577)
                        };
                        let a579 = 0u32;
                        if equal(a578, a579) {
                            let a582 = {
                                let a580 = v0;
                                let a581 = 2u32;
                                fixed_u32::bit_and(a580, a581)
                            };
                            let a583 = 0u32;
                            if equal(a582, a583) {
                                let a586 = {
                                    let a584 = v0;
                                    let a585 = 1u32;
                                    fixed_u32::bit_and(a584, a585)
                                };
                                let a587 = 0u32;
                                if equal(a586, a587) {
                                    Bytes::lit(&[0x30])
                                } else {
                                    Bytes::lit(&[0x31])
                                }
                            } else {
                                let a590 = {
                                    let a588 = v0;
                                    let a589 = 1u32;
                                    fixed_u32::bit_and(a588, a589)
                                };
                                let a591 = 0u32;
                                if equal(a590, a591) {
                                    Bytes::lit(&[0x32])
                                } else {
                                    Bytes::lit(&[0x33])
                                }
                            }
                        } else {
                            let a594 = {
                                let a592 = v0;
                                let a593 = 2u32;
                                fixed_u32::bit_and(a592, a593)
                            };
                            let a595 = 0u32;
                            if equal(a594, a595) {
                                let a598 = {
                                    let a596 = v0;
                                    let a597 = 1u32;
                                    fixed_u32::bit_and(a596, a597)
                                };
                                let a599 = 0u32;
                                if equal(a598, a599) {
                                    Bytes::lit(&[0x34])
                                } else {
                                    Bytes::lit(&[0x35])
                                }
                            } else {
                                let a602 = {
                                    let a600 = v0;
                                    let a601 = 1u32;
                                    fixed_u32::bit_and(a600, a601)
                                };
                                let a603 = 0u32;
                                if equal(a602, a603) {
                                    Bytes::lit(&[0x36])
                                } else {
                                    Bytes::lit(&[0x37])
                                }
                            }
                        }
                    } else {
                        let a606 = {
                            let a604 = v0;
                            let a605 = 4u32;
                            fixed_u32::bit_and(a604, a605)
                        };
                        let a607 = 0u32;
                        if equal(a606, a607) {
                            let a610 = {
                                let a608 = v0;
                                let a609 = 2u32;
                                fixed_u32::bit_and(a608, a609)
                            };
                            let a611 = 0u32;
                            if equal(a610, a611) {
                                let a614 = {
                                    let a612 = v0;
                                    let a613 = 1u32;
                                    fixed_u32::bit_and(a612, a613)
                                };
                                let a615 = 0u32;
                                if equal(a614, a615) {
                                    Bytes::lit(&[0x38])
                                } else {
                                    Bytes::lit(&[0x39])
                                }
                            } else {
                                let a618 = {
                                    let a616 = v0;
                                    let a617 = 1u32;
                                    fixed_u32::bit_and(a616, a617)
                                };
                                let a619 = 0u32;
                                if equal(a618, a619) {
                                    Bytes::lit(&[0x3a])
                                } else {
                                    Bytes::lit(&[0x3b])
                                }
                            }
                        } else {
                            let a622 = {
                                let a620 = v0;
                                let a621 = 2u32;
                                fixed_u32::bit_and(a620, a621)
                            };
                            let a623 = 0u32;
                            if equal(a622, a623) {
                                let a626 = {
                                    let a624 = v0;
                                    let a625 = 1u32;
                                    fixed_u32::bit_and(a624, a625)
                                };
                                let a627 = 0u32;
                                if equal(a626, a627) {
                                    Bytes::lit(&[0x3c])
                                } else {
                                    Bytes::lit(&[0x3d])
                                }
                            } else {
                                let a630 = {
                                    let a628 = v0;
                                    let a629 = 1u32;
                                    fixed_u32::bit_and(a628, a629)
                                };
                                let a631 = 0u32;
                                if equal(a630, a631) {
                                    Bytes::lit(&[0x3e])
                                } else {
                                    Bytes::lit(&[0x3f])
                                }
                            }
                        }
                    }
                }
            }
        } else {
            let a634 = {
                let a632 = v0;
                let a633 = 32u32;
                fixed_u32::bit_and(a632, a633)
            };
            let a635 = 0u32;
            if equal(a634, a635) {
                let a638 = {
                    let a636 = v0;
                    let a637 = 16u32;
                    fixed_u32::bit_and(a636, a637)
                };
                let a639 = 0u32;
                if equal(a638, a639) {
                    let a642 = {
                        let a640 = v0;
                        let a641 = 8u32;
                        fixed_u32::bit_and(a640, a641)
                    };
                    let a643 = 0u32;
                    if equal(a642, a643) {
                        let a646 = {
                            let a644 = v0;
                            let a645 = 4u32;
                            fixed_u32::bit_and(a644, a645)
                        };
                        let a647 = 0u32;
                        if equal(a646, a647) {
                            let a650 = {
                                let a648 = v0;
                                let a649 = 2u32;
                                fixed_u32::bit_and(a648, a649)
                            };
                            let a651 = 0u32;
                            if equal(a650, a651) {
                                let a654 = {
                                    let a652 = v0;
                                    let a653 = 1u32;
                                    fixed_u32::bit_and(a652, a653)
                                };
                                let a655 = 0u32;
                                if equal(a654, a655) {
                                    Bytes::lit(&[0x40])
                                } else {
                                    Bytes::lit(&[0x41])
                                }
                            } else {
                                let a658 = {
                                    let a656 = v0;
                                    let a657 = 1u32;
                                    fixed_u32::bit_and(a656, a657)
                                };
                                let a659 = 0u32;
                                if equal(a658, a659) {
                                    Bytes::lit(&[0x42])
                                } else {
                                    Bytes::lit(&[0x43])
                                }
                            }
                        } else {
                            let a662 = {
                                let a660 = v0;
                                let a661 = 2u32;
                                fixed_u32::bit_and(a660, a661)
                            };
                            let a663 = 0u32;
                            if equal(a662, a663) {
                                let a666 = {
                                    let a664 = v0;
                                    let a665 = 1u32;
                                    fixed_u32::bit_and(a664, a665)
                                };
                                let a667 = 0u32;
                                if equal(a666, a667) {
                                    Bytes::lit(&[0x44])
                                } else {
                                    Bytes::lit(&[0x45])
                                }
                            } else {
                                let a670 = {
                                    let a668 = v0;
                                    let a669 = 1u32;
                                    fixed_u32::bit_and(a668, a669)
                                };
                                let a671 = 0u32;
                                if equal(a670, a671) {
                                    Bytes::lit(&[0x46])
                                } else {
                                    Bytes::lit(&[0x47])
                                }
                            }
                        }
                    } else {
                        let a674 = {
                            let a672 = v0;
                            let a673 = 4u32;
                            fixed_u32::bit_and(a672, a673)
                        };
                        let a675 = 0u32;
                        if equal(a674, a675) {
                            let a678 = {
                                let a676 = v0;
                                let a677 = 2u32;
                                fixed_u32::bit_and(a676, a677)
                            };
                            let a679 = 0u32;
                            if equal(a678, a679) {
                                let a682 = {
                                    let a680 = v0;
                                    let a681 = 1u32;
                                    fixed_u32::bit_and(a680, a681)
                                };
                                let a683 = 0u32;
                                if equal(a682, a683) {
                                    Bytes::lit(&[0x48])
                                } else {
                                    Bytes::lit(&[0x49])
                                }
                            } else {
                                let a686 = {
                                    let a684 = v0;
                                    let a685 = 1u32;
                                    fixed_u32::bit_and(a684, a685)
                                };
                                let a687 = 0u32;
                                if equal(a686, a687) {
                                    Bytes::lit(&[0x4a])
                                } else {
                                    Bytes::lit(&[0x4b])
                                }
                            }
                        } else {
                            let a690 = {
                                let a688 = v0;
                                let a689 = 2u32;
                                fixed_u32::bit_and(a688, a689)
                            };
                            let a691 = 0u32;
                            if equal(a690, a691) {
                                let a694 = {
                                    let a692 = v0;
                                    let a693 = 1u32;
                                    fixed_u32::bit_and(a692, a693)
                                };
                                let a695 = 0u32;
                                if equal(a694, a695) {
                                    Bytes::lit(&[0x4c])
                                } else {
                                    Bytes::lit(&[0x4d])
                                }
                            } else {
                                let a698 = {
                                    let a696 = v0;
                                    let a697 = 1u32;
                                    fixed_u32::bit_and(a696, a697)
                                };
                                let a699 = 0u32;
                                if equal(a698, a699) {
                                    Bytes::lit(&[0x4e])
                                } else {
                                    Bytes::lit(&[0x4f])
                                }
                            }
                        }
                    }
                } else {
                    let a702 = {
                        let a700 = v0;
                        let a701 = 8u32;
                        fixed_u32::bit_and(a700, a701)
                    };
                    let a703 = 0u32;
                    if equal(a702, a703) {
                        let a706 = {
                            let a704 = v0;
                            let a705 = 4u32;
                            fixed_u32::bit_and(a704, a705)
                        };
                        let a707 = 0u32;
                        if equal(a706, a707) {
                            let a710 = {
                                let a708 = v0;
                                let a709 = 2u32;
                                fixed_u32::bit_and(a708, a709)
                            };
                            let a711 = 0u32;
                            if equal(a710, a711) {
                                let a714 = {
                                    let a712 = v0;
                                    let a713 = 1u32;
                                    fixed_u32::bit_and(a712, a713)
                                };
                                let a715 = 0u32;
                                if equal(a714, a715) {
                                    Bytes::lit(&[0x50])
                                } else {
                                    Bytes::lit(&[0x51])
                                }
                            } else {
                                let a718 = {
                                    let a716 = v0;
                                    let a717 = 1u32;
                                    fixed_u32::bit_and(a716, a717)
                                };
                                let a719 = 0u32;
                                if equal(a718, a719) {
                                    Bytes::lit(&[0x52])
                                } else {
                                    Bytes::lit(&[0x53])
                                }
                            }
                        } else {
                            let a722 = {
                                let a720 = v0;
                                let a721 = 2u32;
                                fixed_u32::bit_and(a720, a721)
                            };
                            let a723 = 0u32;
                            if equal(a722, a723) {
                                let a726 = {
                                    let a724 = v0;
                                    let a725 = 1u32;
                                    fixed_u32::bit_and(a724, a725)
                                };
                                let a727 = 0u32;
                                if equal(a726, a727) {
                                    Bytes::lit(&[0x54])
                                } else {
                                    Bytes::lit(&[0x55])
                                }
                            } else {
                                let a730 = {
                                    let a728 = v0;
                                    let a729 = 1u32;
                                    fixed_u32::bit_and(a728, a729)
                                };
                                let a731 = 0u32;
                                if equal(a730, a731) {
                                    Bytes::lit(&[0x56])
                                } else {
                                    Bytes::lit(&[0x57])
                                }
                            }
                        }
                    } else {
                        let a734 = {
                            let a732 = v0;
                            let a733 = 4u32;
                            fixed_u32::bit_and(a732, a733)
                        };
                        let a735 = 0u32;
                        if equal(a734, a735) {
                            let a738 = {
                                let a736 = v0;
                                let a737 = 2u32;
                                fixed_u32::bit_and(a736, a737)
                            };
                            let a739 = 0u32;
                            if equal(a738, a739) {
                                let a742 = {
                                    let a740 = v0;
                                    let a741 = 1u32;
                                    fixed_u32::bit_and(a740, a741)
                                };
                                let a743 = 0u32;
                                if equal(a742, a743) {
                                    Bytes::lit(&[0x58])
                                } else {
                                    Bytes::lit(&[0x59])
                                }
                            } else {
                                let a746 = {
                                    let a744 = v0;
                                    let a745 = 1u32;
                                    fixed_u32::bit_and(a744, a745)
                                };
                                let a747 = 0u32;
                                if equal(a746, a747) {
                                    Bytes::lit(&[0x5a])
                                } else {
                                    Bytes::lit(&[0x5b])
                                }
                            }
                        } else {
                            let a750 = {
                                let a748 = v0;
                                let a749 = 2u32;
                                fixed_u32::bit_and(a748, a749)
                            };
                            let a751 = 0u32;
                            if equal(a750, a751) {
                                let a754 = {
                                    let a752 = v0;
                                    let a753 = 1u32;
                                    fixed_u32::bit_and(a752, a753)
                                };
                                let a755 = 0u32;
                                if equal(a754, a755) {
                                    Bytes::lit(&[0x5c])
                                } else {
                                    Bytes::lit(&[0x5d])
                                }
                            } else {
                                let a758 = {
                                    let a756 = v0;
                                    let a757 = 1u32;
                                    fixed_u32::bit_and(a756, a757)
                                };
                                let a759 = 0u32;
                                if equal(a758, a759) {
                                    Bytes::lit(&[0x5e])
                                } else {
                                    Bytes::lit(&[0x5f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a762 = {
                    let a760 = v0;
                    let a761 = 16u32;
                    fixed_u32::bit_and(a760, a761)
                };
                let a763 = 0u32;
                if equal(a762, a763) {
                    let a766 = {
                        let a764 = v0;
                        let a765 = 8u32;
                        fixed_u32::bit_and(a764, a765)
                    };
                    let a767 = 0u32;
                    if equal(a766, a767) {
                        let a770 = {
                            let a768 = v0;
                            let a769 = 4u32;
                            fixed_u32::bit_and(a768, a769)
                        };
                        let a771 = 0u32;
                        if equal(a770, a771) {
                            let a774 = {
                                let a772 = v0;
                                let a773 = 2u32;
                                fixed_u32::bit_and(a772, a773)
                            };
                            let a775 = 0u32;
                            if equal(a774, a775) {
                                let a778 = {
                                    let a776 = v0;
                                    let a777 = 1u32;
                                    fixed_u32::bit_and(a776, a777)
                                };
                                let a779 = 0u32;
                                if equal(a778, a779) {
                                    Bytes::lit(&[0x60])
                                } else {
                                    Bytes::lit(&[0x61])
                                }
                            } else {
                                let a782 = {
                                    let a780 = v0;
                                    let a781 = 1u32;
                                    fixed_u32::bit_and(a780, a781)
                                };
                                let a783 = 0u32;
                                if equal(a782, a783) {
                                    Bytes::lit(&[0x62])
                                } else {
                                    Bytes::lit(&[0x63])
                                }
                            }
                        } else {
                            let a786 = {
                                let a784 = v0;
                                let a785 = 2u32;
                                fixed_u32::bit_and(a784, a785)
                            };
                            let a787 = 0u32;
                            if equal(a786, a787) {
                                let a790 = {
                                    let a788 = v0;
                                    let a789 = 1u32;
                                    fixed_u32::bit_and(a788, a789)
                                };
                                let a791 = 0u32;
                                if equal(a790, a791) {
                                    Bytes::lit(&[0x64])
                                } else {
                                    Bytes::lit(&[0x65])
                                }
                            } else {
                                let a794 = {
                                    let a792 = v0;
                                    let a793 = 1u32;
                                    fixed_u32::bit_and(a792, a793)
                                };
                                let a795 = 0u32;
                                if equal(a794, a795) {
                                    Bytes::lit(&[0x66])
                                } else {
                                    Bytes::lit(&[0x67])
                                }
                            }
                        }
                    } else {
                        let a798 = {
                            let a796 = v0;
                            let a797 = 4u32;
                            fixed_u32::bit_and(a796, a797)
                        };
                        let a799 = 0u32;
                        if equal(a798, a799) {
                            let a802 = {
                                let a800 = v0;
                                let a801 = 2u32;
                                fixed_u32::bit_and(a800, a801)
                            };
                            let a803 = 0u32;
                            if equal(a802, a803) {
                                let a806 = {
                                    let a804 = v0;
                                    let a805 = 1u32;
                                    fixed_u32::bit_and(a804, a805)
                                };
                                let a807 = 0u32;
                                if equal(a806, a807) {
                                    Bytes::lit(&[0x68])
                                } else {
                                    Bytes::lit(&[0x69])
                                }
                            } else {
                                let a810 = {
                                    let a808 = v0;
                                    let a809 = 1u32;
                                    fixed_u32::bit_and(a808, a809)
                                };
                                let a811 = 0u32;
                                if equal(a810, a811) {
                                    Bytes::lit(&[0x6a])
                                } else {
                                    Bytes::lit(&[0x6b])
                                }
                            }
                        } else {
                            let a814 = {
                                let a812 = v0;
                                let a813 = 2u32;
                                fixed_u32::bit_and(a812, a813)
                            };
                            let a815 = 0u32;
                            if equal(a814, a815) {
                                let a818 = {
                                    let a816 = v0;
                                    let a817 = 1u32;
                                    fixed_u32::bit_and(a816, a817)
                                };
                                let a819 = 0u32;
                                if equal(a818, a819) {
                                    Bytes::lit(&[0x6c])
                                } else {
                                    Bytes::lit(&[0x6d])
                                }
                            } else {
                                let a822 = {
                                    let a820 = v0;
                                    let a821 = 1u32;
                                    fixed_u32::bit_and(a820, a821)
                                };
                                let a823 = 0u32;
                                if equal(a822, a823) {
                                    Bytes::lit(&[0x6e])
                                } else {
                                    Bytes::lit(&[0x6f])
                                }
                            }
                        }
                    }
                } else {
                    let a826 = {
                        let a824 = v0;
                        let a825 = 8u32;
                        fixed_u32::bit_and(a824, a825)
                    };
                    let a827 = 0u32;
                    if equal(a826, a827) {
                        let a830 = {
                            let a828 = v0;
                            let a829 = 4u32;
                            fixed_u32::bit_and(a828, a829)
                        };
                        let a831 = 0u32;
                        if equal(a830, a831) {
                            let a834 = {
                                let a832 = v0;
                                let a833 = 2u32;
                                fixed_u32::bit_and(a832, a833)
                            };
                            let a835 = 0u32;
                            if equal(a834, a835) {
                                let a838 = {
                                    let a836 = v0;
                                    let a837 = 1u32;
                                    fixed_u32::bit_and(a836, a837)
                                };
                                let a839 = 0u32;
                                if equal(a838, a839) {
                                    Bytes::lit(&[0x70])
                                } else {
                                    Bytes::lit(&[0x71])
                                }
                            } else {
                                let a842 = {
                                    let a840 = v0;
                                    let a841 = 1u32;
                                    fixed_u32::bit_and(a840, a841)
                                };
                                let a843 = 0u32;
                                if equal(a842, a843) {
                                    Bytes::lit(&[0x72])
                                } else {
                                    Bytes::lit(&[0x73])
                                }
                            }
                        } else {
                            let a846 = {
                                let a844 = v0;
                                let a845 = 2u32;
                                fixed_u32::bit_and(a844, a845)
                            };
                            let a847 = 0u32;
                            if equal(a846, a847) {
                                let a850 = {
                                    let a848 = v0;
                                    let a849 = 1u32;
                                    fixed_u32::bit_and(a848, a849)
                                };
                                let a851 = 0u32;
                                if equal(a850, a851) {
                                    Bytes::lit(&[0x74])
                                } else {
                                    Bytes::lit(&[0x75])
                                }
                            } else {
                                let a854 = {
                                    let a852 = v0;
                                    let a853 = 1u32;
                                    fixed_u32::bit_and(a852, a853)
                                };
                                let a855 = 0u32;
                                if equal(a854, a855) {
                                    Bytes::lit(&[0x76])
                                } else {
                                    Bytes::lit(&[0x77])
                                }
                            }
                        }
                    } else {
                        let a858 = {
                            let a856 = v0;
                            let a857 = 4u32;
                            fixed_u32::bit_and(a856, a857)
                        };
                        let a859 = 0u32;
                        if equal(a858, a859) {
                            let a862 = {
                                let a860 = v0;
                                let a861 = 2u32;
                                fixed_u32::bit_and(a860, a861)
                            };
                            let a863 = 0u32;
                            if equal(a862, a863) {
                                let a866 = {
                                    let a864 = v0;
                                    let a865 = 1u32;
                                    fixed_u32::bit_and(a864, a865)
                                };
                                let a867 = 0u32;
                                if equal(a866, a867) {
                                    Bytes::lit(&[0x78])
                                } else {
                                    Bytes::lit(&[0x79])
                                }
                            } else {
                                let a870 = {
                                    let a868 = v0;
                                    let a869 = 1u32;
                                    fixed_u32::bit_and(a868, a869)
                                };
                                let a871 = 0u32;
                                if equal(a870, a871) {
                                    Bytes::lit(&[0x7a])
                                } else {
                                    Bytes::lit(&[0x7b])
                                }
                            }
                        } else {
                            let a874 = {
                                let a872 = v0;
                                let a873 = 2u32;
                                fixed_u32::bit_and(a872, a873)
                            };
                            let a875 = 0u32;
                            if equal(a874, a875) {
                                let a878 = {
                                    let a876 = v0;
                                    let a877 = 1u32;
                                    fixed_u32::bit_and(a876, a877)
                                };
                                let a879 = 0u32;
                                if equal(a878, a879) {
                                    Bytes::lit(&[0x7c])
                                } else {
                                    Bytes::lit(&[0x7d])
                                }
                            } else {
                                let a882 = {
                                    let a880 = v0;
                                    let a881 = 1u32;
                                    fixed_u32::bit_and(a880, a881)
                                };
                                let a883 = 0u32;
                                if equal(a882, a883) {
                                    Bytes::lit(&[0x7e])
                                } else {
                                    Bytes::lit(&[0x7f])
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        let a886 = {
            let a884 = v0;
            let a885 = 64u32;
            fixed_u32::bit_and(a884, a885)
        };
        let a887 = 0u32;
        if equal(a886, a887) {
            let a890 = {
                let a888 = v0;
                let a889 = 32u32;
                fixed_u32::bit_and(a888, a889)
            };
            let a891 = 0u32;
            if equal(a890, a891) {
                let a894 = {
                    let a892 = v0;
                    let a893 = 16u32;
                    fixed_u32::bit_and(a892, a893)
                };
                let a895 = 0u32;
                if equal(a894, a895) {
                    let a898 = {
                        let a896 = v0;
                        let a897 = 8u32;
                        fixed_u32::bit_and(a896, a897)
                    };
                    let a899 = 0u32;
                    if equal(a898, a899) {
                        let a902 = {
                            let a900 = v0;
                            let a901 = 4u32;
                            fixed_u32::bit_and(a900, a901)
                        };
                        let a903 = 0u32;
                        if equal(a902, a903) {
                            let a906 = {
                                let a904 = v0;
                                let a905 = 2u32;
                                fixed_u32::bit_and(a904, a905)
                            };
                            let a907 = 0u32;
                            if equal(a906, a907) {
                                let a910 = {
                                    let a908 = v0;
                                    let a909 = 1u32;
                                    fixed_u32::bit_and(a908, a909)
                                };
                                let a911 = 0u32;
                                if equal(a910, a911) {
                                    Bytes::lit(&[0x80])
                                } else {
                                    Bytes::lit(&[0x81])
                                }
                            } else {
                                let a914 = {
                                    let a912 = v0;
                                    let a913 = 1u32;
                                    fixed_u32::bit_and(a912, a913)
                                };
                                let a915 = 0u32;
                                if equal(a914, a915) {
                                    Bytes::lit(&[0x82])
                                } else {
                                    Bytes::lit(&[0x83])
                                }
                            }
                        } else {
                            let a918 = {
                                let a916 = v0;
                                let a917 = 2u32;
                                fixed_u32::bit_and(a916, a917)
                            };
                            let a919 = 0u32;
                            if equal(a918, a919) {
                                let a922 = {
                                    let a920 = v0;
                                    let a921 = 1u32;
                                    fixed_u32::bit_and(a920, a921)
                                };
                                let a923 = 0u32;
                                if equal(a922, a923) {
                                    Bytes::lit(&[0x84])
                                } else {
                                    Bytes::lit(&[0x85])
                                }
                            } else {
                                let a926 = {
                                    let a924 = v0;
                                    let a925 = 1u32;
                                    fixed_u32::bit_and(a924, a925)
                                };
                                let a927 = 0u32;
                                if equal(a926, a927) {
                                    Bytes::lit(&[0x86])
                                } else {
                                    Bytes::lit(&[0x87])
                                }
                            }
                        }
                    } else {
                        let a930 = {
                            let a928 = v0;
                            let a929 = 4u32;
                            fixed_u32::bit_and(a928, a929)
                        };
                        let a931 = 0u32;
                        if equal(a930, a931) {
                            let a934 = {
                                let a932 = v0;
                                let a933 = 2u32;
                                fixed_u32::bit_and(a932, a933)
                            };
                            let a935 = 0u32;
                            if equal(a934, a935) {
                                let a938 = {
                                    let a936 = v0;
                                    let a937 = 1u32;
                                    fixed_u32::bit_and(a936, a937)
                                };
                                let a939 = 0u32;
                                if equal(a938, a939) {
                                    Bytes::lit(&[0x88])
                                } else {
                                    Bytes::lit(&[0x89])
                                }
                            } else {
                                let a942 = {
                                    let a940 = v0;
                                    let a941 = 1u32;
                                    fixed_u32::bit_and(a940, a941)
                                };
                                let a943 = 0u32;
                                if equal(a942, a943) {
                                    Bytes::lit(&[0x8a])
                                } else {
                                    Bytes::lit(&[0x8b])
                                }
                            }
                        } else {
                            let a946 = {
                                let a944 = v0;
                                let a945 = 2u32;
                                fixed_u32::bit_and(a944, a945)
                            };
                            let a947 = 0u32;
                            if equal(a946, a947) {
                                let a950 = {
                                    let a948 = v0;
                                    let a949 = 1u32;
                                    fixed_u32::bit_and(a948, a949)
                                };
                                let a951 = 0u32;
                                if equal(a950, a951) {
                                    Bytes::lit(&[0x8c])
                                } else {
                                    Bytes::lit(&[0x8d])
                                }
                            } else {
                                let a954 = {
                                    let a952 = v0;
                                    let a953 = 1u32;
                                    fixed_u32::bit_and(a952, a953)
                                };
                                let a955 = 0u32;
                                if equal(a954, a955) {
                                    Bytes::lit(&[0x8e])
                                } else {
                                    Bytes::lit(&[0x8f])
                                }
                            }
                        }
                    }
                } else {
                    let a958 = {
                        let a956 = v0;
                        let a957 = 8u32;
                        fixed_u32::bit_and(a956, a957)
                    };
                    let a959 = 0u32;
                    if equal(a958, a959) {
                        let a962 = {
                            let a960 = v0;
                            let a961 = 4u32;
                            fixed_u32::bit_and(a960, a961)
                        };
                        let a963 = 0u32;
                        if equal(a962, a963) {
                            let a966 = {
                                let a964 = v0;
                                let a965 = 2u32;
                                fixed_u32::bit_and(a964, a965)
                            };
                            let a967 = 0u32;
                            if equal(a966, a967) {
                                let a970 = {
                                    let a968 = v0;
                                    let a969 = 1u32;
                                    fixed_u32::bit_and(a968, a969)
                                };
                                let a971 = 0u32;
                                if equal(a970, a971) {
                                    Bytes::lit(&[0x90])
                                } else {
                                    Bytes::lit(&[0x91])
                                }
                            } else {
                                let a974 = {
                                    let a972 = v0;
                                    let a973 = 1u32;
                                    fixed_u32::bit_and(a972, a973)
                                };
                                let a975 = 0u32;
                                if equal(a974, a975) {
                                    Bytes::lit(&[0x92])
                                } else {
                                    Bytes::lit(&[0x93])
                                }
                            }
                        } else {
                            let a978 = {
                                let a976 = v0;
                                let a977 = 2u32;
                                fixed_u32::bit_and(a976, a977)
                            };
                            let a979 = 0u32;
                            if equal(a978, a979) {
                                let a982 = {
                                    let a980 = v0;
                                    let a981 = 1u32;
                                    fixed_u32::bit_and(a980, a981)
                                };
                                let a983 = 0u32;
                                if equal(a982, a983) {
                                    Bytes::lit(&[0x94])
                                } else {
                                    Bytes::lit(&[0x95])
                                }
                            } else {
                                let a986 = {
                                    let a984 = v0;
                                    let a985 = 1u32;
                                    fixed_u32::bit_and(a984, a985)
                                };
                                let a987 = 0u32;
                                if equal(a986, a987) {
                                    Bytes::lit(&[0x96])
                                } else {
                                    Bytes::lit(&[0x97])
                                }
                            }
                        }
                    } else {
                        let a990 = {
                            let a988 = v0;
                            let a989 = 4u32;
                            fixed_u32::bit_and(a988, a989)
                        };
                        let a991 = 0u32;
                        if equal(a990, a991) {
                            let a994 = {
                                let a992 = v0;
                                let a993 = 2u32;
                                fixed_u32::bit_and(a992, a993)
                            };
                            let a995 = 0u32;
                            if equal(a994, a995) {
                                let a998 = {
                                    let a996 = v0;
                                    let a997 = 1u32;
                                    fixed_u32::bit_and(a996, a997)
                                };
                                let a999 = 0u32;
                                if equal(a998, a999) {
                                    Bytes::lit(&[0x98])
                                } else {
                                    Bytes::lit(&[0x99])
                                }
                            } else {
                                let a1002 = {
                                    let a1000 = v0;
                                    let a1001 = 1u32;
                                    fixed_u32::bit_and(a1000, a1001)
                                };
                                let a1003 = 0u32;
                                if equal(a1002, a1003) {
                                    Bytes::lit(&[0x9a])
                                } else {
                                    Bytes::lit(&[0x9b])
                                }
                            }
                        } else {
                            let a1006 = {
                                let a1004 = v0;
                                let a1005 = 2u32;
                                fixed_u32::bit_and(a1004, a1005)
                            };
                            let a1007 = 0u32;
                            if equal(a1006, a1007) {
                                let a1010 = {
                                    let a1008 = v0;
                                    let a1009 = 1u32;
                                    fixed_u32::bit_and(a1008, a1009)
                                };
                                let a1011 = 0u32;
                                if equal(a1010, a1011) {
                                    Bytes::lit(&[0x9c])
                                } else {
                                    Bytes::lit(&[0x9d])
                                }
                            } else {
                                let a1014 = {
                                    let a1012 = v0;
                                    let a1013 = 1u32;
                                    fixed_u32::bit_and(a1012, a1013)
                                };
                                let a1015 = 0u32;
                                if equal(a1014, a1015) {
                                    Bytes::lit(&[0x9e])
                                } else {
                                    Bytes::lit(&[0x9f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a1018 = {
                    let a1016 = v0;
                    let a1017 = 16u32;
                    fixed_u32::bit_and(a1016, a1017)
                };
                let a1019 = 0u32;
                if equal(a1018, a1019) {
                    let a1022 = {
                        let a1020 = v0;
                        let a1021 = 8u32;
                        fixed_u32::bit_and(a1020, a1021)
                    };
                    let a1023 = 0u32;
                    if equal(a1022, a1023) {
                        let a1026 = {
                            let a1024 = v0;
                            let a1025 = 4u32;
                            fixed_u32::bit_and(a1024, a1025)
                        };
                        let a1027 = 0u32;
                        if equal(a1026, a1027) {
                            let a1030 = {
                                let a1028 = v0;
                                let a1029 = 2u32;
                                fixed_u32::bit_and(a1028, a1029)
                            };
                            let a1031 = 0u32;
                            if equal(a1030, a1031) {
                                let a1034 = {
                                    let a1032 = v0;
                                    let a1033 = 1u32;
                                    fixed_u32::bit_and(a1032, a1033)
                                };
                                let a1035 = 0u32;
                                if equal(a1034, a1035) {
                                    Bytes::lit(&[0xa0])
                                } else {
                                    Bytes::lit(&[0xa1])
                                }
                            } else {
                                let a1038 = {
                                    let a1036 = v0;
                                    let a1037 = 1u32;
                                    fixed_u32::bit_and(a1036, a1037)
                                };
                                let a1039 = 0u32;
                                if equal(a1038, a1039) {
                                    Bytes::lit(&[0xa2])
                                } else {
                                    Bytes::lit(&[0xa3])
                                }
                            }
                        } else {
                            let a1042 = {
                                let a1040 = v0;
                                let a1041 = 2u32;
                                fixed_u32::bit_and(a1040, a1041)
                            };
                            let a1043 = 0u32;
                            if equal(a1042, a1043) {
                                let a1046 = {
                                    let a1044 = v0;
                                    let a1045 = 1u32;
                                    fixed_u32::bit_and(a1044, a1045)
                                };
                                let a1047 = 0u32;
                                if equal(a1046, a1047) {
                                    Bytes::lit(&[0xa4])
                                } else {
                                    Bytes::lit(&[0xa5])
                                }
                            } else {
                                let a1050 = {
                                    let a1048 = v0;
                                    let a1049 = 1u32;
                                    fixed_u32::bit_and(a1048, a1049)
                                };
                                let a1051 = 0u32;
                                if equal(a1050, a1051) {
                                    Bytes::lit(&[0xa6])
                                } else {
                                    Bytes::lit(&[0xa7])
                                }
                            }
                        }
                    } else {
                        let a1054 = {
                            let a1052 = v0;
                            let a1053 = 4u32;
                            fixed_u32::bit_and(a1052, a1053)
                        };
                        let a1055 = 0u32;
                        if equal(a1054, a1055) {
                            let a1058 = {
                                let a1056 = v0;
                                let a1057 = 2u32;
                                fixed_u32::bit_and(a1056, a1057)
                            };
                            let a1059 = 0u32;
                            if equal(a1058, a1059) {
                                let a1062 = {
                                    let a1060 = v0;
                                    let a1061 = 1u32;
                                    fixed_u32::bit_and(a1060, a1061)
                                };
                                let a1063 = 0u32;
                                if equal(a1062, a1063) {
                                    Bytes::lit(&[0xa8])
                                } else {
                                    Bytes::lit(&[0xa9])
                                }
                            } else {
                                let a1066 = {
                                    let a1064 = v0;
                                    let a1065 = 1u32;
                                    fixed_u32::bit_and(a1064, a1065)
                                };
                                let a1067 = 0u32;
                                if equal(a1066, a1067) {
                                    Bytes::lit(&[0xaa])
                                } else {
                                    Bytes::lit(&[0xab])
                                }
                            }
                        } else {
                            let a1070 = {
                                let a1068 = v0;
                                let a1069 = 2u32;
                                fixed_u32::bit_and(a1068, a1069)
                            };
                            let a1071 = 0u32;
                            if equal(a1070, a1071) {
                                let a1074 = {
                                    let a1072 = v0;
                                    let a1073 = 1u32;
                                    fixed_u32::bit_and(a1072, a1073)
                                };
                                let a1075 = 0u32;
                                if equal(a1074, a1075) {
                                    Bytes::lit(&[0xac])
                                } else {
                                    Bytes::lit(&[0xad])
                                }
                            } else {
                                let a1078 = {
                                    let a1076 = v0;
                                    let a1077 = 1u32;
                                    fixed_u32::bit_and(a1076, a1077)
                                };
                                let a1079 = 0u32;
                                if equal(a1078, a1079) {
                                    Bytes::lit(&[0xae])
                                } else {
                                    Bytes::lit(&[0xaf])
                                }
                            }
                        }
                    }
                } else {
                    let a1082 = {
                        let a1080 = v0;
                        let a1081 = 8u32;
                        fixed_u32::bit_and(a1080, a1081)
                    };
                    let a1083 = 0u32;
                    if equal(a1082, a1083) {
                        let a1086 = {
                            let a1084 = v0;
                            let a1085 = 4u32;
                            fixed_u32::bit_and(a1084, a1085)
                        };
                        let a1087 = 0u32;
                        if equal(a1086, a1087) {
                            let a1090 = {
                                let a1088 = v0;
                                let a1089 = 2u32;
                                fixed_u32::bit_and(a1088, a1089)
                            };
                            let a1091 = 0u32;
                            if equal(a1090, a1091) {
                                let a1094 = {
                                    let a1092 = v0;
                                    let a1093 = 1u32;
                                    fixed_u32::bit_and(a1092, a1093)
                                };
                                let a1095 = 0u32;
                                if equal(a1094, a1095) {
                                    Bytes::lit(&[0xb0])
                                } else {
                                    Bytes::lit(&[0xb1])
                                }
                            } else {
                                let a1098 = {
                                    let a1096 = v0;
                                    let a1097 = 1u32;
                                    fixed_u32::bit_and(a1096, a1097)
                                };
                                let a1099 = 0u32;
                                if equal(a1098, a1099) {
                                    Bytes::lit(&[0xb2])
                                } else {
                                    Bytes::lit(&[0xb3])
                                }
                            }
                        } else {
                            let a1102 = {
                                let a1100 = v0;
                                let a1101 = 2u32;
                                fixed_u32::bit_and(a1100, a1101)
                            };
                            let a1103 = 0u32;
                            if equal(a1102, a1103) {
                                let a1106 = {
                                    let a1104 = v0;
                                    let a1105 = 1u32;
                                    fixed_u32::bit_and(a1104, a1105)
                                };
                                let a1107 = 0u32;
                                if equal(a1106, a1107) {
                                    Bytes::lit(&[0xb4])
                                } else {
                                    Bytes::lit(&[0xb5])
                                }
                            } else {
                                let a1110 = {
                                    let a1108 = v0;
                                    let a1109 = 1u32;
                                    fixed_u32::bit_and(a1108, a1109)
                                };
                                let a1111 = 0u32;
                                if equal(a1110, a1111) {
                                    Bytes::lit(&[0xb6])
                                } else {
                                    Bytes::lit(&[0xb7])
                                }
                            }
                        }
                    } else {
                        let a1114 = {
                            let a1112 = v0;
                            let a1113 = 4u32;
                            fixed_u32::bit_and(a1112, a1113)
                        };
                        let a1115 = 0u32;
                        if equal(a1114, a1115) {
                            let a1118 = {
                                let a1116 = v0;
                                let a1117 = 2u32;
                                fixed_u32::bit_and(a1116, a1117)
                            };
                            let a1119 = 0u32;
                            if equal(a1118, a1119) {
                                let a1122 = {
                                    let a1120 = v0;
                                    let a1121 = 1u32;
                                    fixed_u32::bit_and(a1120, a1121)
                                };
                                let a1123 = 0u32;
                                if equal(a1122, a1123) {
                                    Bytes::lit(&[0xb8])
                                } else {
                                    Bytes::lit(&[0xb9])
                                }
                            } else {
                                let a1126 = {
                                    let a1124 = v0;
                                    let a1125 = 1u32;
                                    fixed_u32::bit_and(a1124, a1125)
                                };
                                let a1127 = 0u32;
                                if equal(a1126, a1127) {
                                    Bytes::lit(&[0xba])
                                } else {
                                    Bytes::lit(&[0xbb])
                                }
                            }
                        } else {
                            let a1130 = {
                                let a1128 = v0;
                                let a1129 = 2u32;
                                fixed_u32::bit_and(a1128, a1129)
                            };
                            let a1131 = 0u32;
                            if equal(a1130, a1131) {
                                let a1134 = {
                                    let a1132 = v0;
                                    let a1133 = 1u32;
                                    fixed_u32::bit_and(a1132, a1133)
                                };
                                let a1135 = 0u32;
                                if equal(a1134, a1135) {
                                    Bytes::lit(&[0xbc])
                                } else {
                                    Bytes::lit(&[0xbd])
                                }
                            } else {
                                let a1138 = {
                                    let a1136 = v0;
                                    let a1137 = 1u32;
                                    fixed_u32::bit_and(a1136, a1137)
                                };
                                let a1139 = 0u32;
                                if equal(a1138, a1139) {
                                    Bytes::lit(&[0xbe])
                                } else {
                                    Bytes::lit(&[0xbf])
                                }
                            }
                        }
                    }
                }
            }
        } else {
            let a1142 = {
                let a1140 = v0;
                let a1141 = 32u32;
                fixed_u32::bit_and(a1140, a1141)
            };
            let a1143 = 0u32;
            if equal(a1142, a1143) {
                let a1146 = {
                    let a1144 = v0;
                    let a1145 = 16u32;
                    fixed_u32::bit_and(a1144, a1145)
                };
                let a1147 = 0u32;
                if equal(a1146, a1147) {
                    let a1150 = {
                        let a1148 = v0;
                        let a1149 = 8u32;
                        fixed_u32::bit_and(a1148, a1149)
                    };
                    let a1151 = 0u32;
                    if equal(a1150, a1151) {
                        let a1154 = {
                            let a1152 = v0;
                            let a1153 = 4u32;
                            fixed_u32::bit_and(a1152, a1153)
                        };
                        let a1155 = 0u32;
                        if equal(a1154, a1155) {
                            let a1158 = {
                                let a1156 = v0;
                                let a1157 = 2u32;
                                fixed_u32::bit_and(a1156, a1157)
                            };
                            let a1159 = 0u32;
                            if equal(a1158, a1159) {
                                let a1162 = {
                                    let a1160 = v0;
                                    let a1161 = 1u32;
                                    fixed_u32::bit_and(a1160, a1161)
                                };
                                let a1163 = 0u32;
                                if equal(a1162, a1163) {
                                    Bytes::lit(&[0xc0])
                                } else {
                                    Bytes::lit(&[0xc1])
                                }
                            } else {
                                let a1166 = {
                                    let a1164 = v0;
                                    let a1165 = 1u32;
                                    fixed_u32::bit_and(a1164, a1165)
                                };
                                let a1167 = 0u32;
                                if equal(a1166, a1167) {
                                    Bytes::lit(&[0xc2])
                                } else {
                                    Bytes::lit(&[0xc3])
                                }
                            }
                        } else {
                            let a1170 = {
                                let a1168 = v0;
                                let a1169 = 2u32;
                                fixed_u32::bit_and(a1168, a1169)
                            };
                            let a1171 = 0u32;
                            if equal(a1170, a1171) {
                                let a1174 = {
                                    let a1172 = v0;
                                    let a1173 = 1u32;
                                    fixed_u32::bit_and(a1172, a1173)
                                };
                                let a1175 = 0u32;
                                if equal(a1174, a1175) {
                                    Bytes::lit(&[0xc4])
                                } else {
                                    Bytes::lit(&[0xc5])
                                }
                            } else {
                                let a1178 = {
                                    let a1176 = v0;
                                    let a1177 = 1u32;
                                    fixed_u32::bit_and(a1176, a1177)
                                };
                                let a1179 = 0u32;
                                if equal(a1178, a1179) {
                                    Bytes::lit(&[0xc6])
                                } else {
                                    Bytes::lit(&[0xc7])
                                }
                            }
                        }
                    } else {
                        let a1182 = {
                            let a1180 = v0;
                            let a1181 = 4u32;
                            fixed_u32::bit_and(a1180, a1181)
                        };
                        let a1183 = 0u32;
                        if equal(a1182, a1183) {
                            let a1186 = {
                                let a1184 = v0;
                                let a1185 = 2u32;
                                fixed_u32::bit_and(a1184, a1185)
                            };
                            let a1187 = 0u32;
                            if equal(a1186, a1187) {
                                let a1190 = {
                                    let a1188 = v0;
                                    let a1189 = 1u32;
                                    fixed_u32::bit_and(a1188, a1189)
                                };
                                let a1191 = 0u32;
                                if equal(a1190, a1191) {
                                    Bytes::lit(&[0xc8])
                                } else {
                                    Bytes::lit(&[0xc9])
                                }
                            } else {
                                let a1194 = {
                                    let a1192 = v0;
                                    let a1193 = 1u32;
                                    fixed_u32::bit_and(a1192, a1193)
                                };
                                let a1195 = 0u32;
                                if equal(a1194, a1195) {
                                    Bytes::lit(&[0xca])
                                } else {
                                    Bytes::lit(&[0xcb])
                                }
                            }
                        } else {
                            let a1198 = {
                                let a1196 = v0;
                                let a1197 = 2u32;
                                fixed_u32::bit_and(a1196, a1197)
                            };
                            let a1199 = 0u32;
                            if equal(a1198, a1199) {
                                let a1202 = {
                                    let a1200 = v0;
                                    let a1201 = 1u32;
                                    fixed_u32::bit_and(a1200, a1201)
                                };
                                let a1203 = 0u32;
                                if equal(a1202, a1203) {
                                    Bytes::lit(&[0xcc])
                                } else {
                                    Bytes::lit(&[0xcd])
                                }
                            } else {
                                let a1206 = {
                                    let a1204 = v0;
                                    let a1205 = 1u32;
                                    fixed_u32::bit_and(a1204, a1205)
                                };
                                let a1207 = 0u32;
                                if equal(a1206, a1207) {
                                    Bytes::lit(&[0xce])
                                } else {
                                    Bytes::lit(&[0xcf])
                                }
                            }
                        }
                    }
                } else {
                    let a1210 = {
                        let a1208 = v0;
                        let a1209 = 8u32;
                        fixed_u32::bit_and(a1208, a1209)
                    };
                    let a1211 = 0u32;
                    if equal(a1210, a1211) {
                        let a1214 = {
                            let a1212 = v0;
                            let a1213 = 4u32;
                            fixed_u32::bit_and(a1212, a1213)
                        };
                        let a1215 = 0u32;
                        if equal(a1214, a1215) {
                            let a1218 = {
                                let a1216 = v0;
                                let a1217 = 2u32;
                                fixed_u32::bit_and(a1216, a1217)
                            };
                            let a1219 = 0u32;
                            if equal(a1218, a1219) {
                                let a1222 = {
                                    let a1220 = v0;
                                    let a1221 = 1u32;
                                    fixed_u32::bit_and(a1220, a1221)
                                };
                                let a1223 = 0u32;
                                if equal(a1222, a1223) {
                                    Bytes::lit(&[0xd0])
                                } else {
                                    Bytes::lit(&[0xd1])
                                }
                            } else {
                                let a1226 = {
                                    let a1224 = v0;
                                    let a1225 = 1u32;
                                    fixed_u32::bit_and(a1224, a1225)
                                };
                                let a1227 = 0u32;
                                if equal(a1226, a1227) {
                                    Bytes::lit(&[0xd2])
                                } else {
                                    Bytes::lit(&[0xd3])
                                }
                            }
                        } else {
                            let a1230 = {
                                let a1228 = v0;
                                let a1229 = 2u32;
                                fixed_u32::bit_and(a1228, a1229)
                            };
                            let a1231 = 0u32;
                            if equal(a1230, a1231) {
                                let a1234 = {
                                    let a1232 = v0;
                                    let a1233 = 1u32;
                                    fixed_u32::bit_and(a1232, a1233)
                                };
                                let a1235 = 0u32;
                                if equal(a1234, a1235) {
                                    Bytes::lit(&[0xd4])
                                } else {
                                    Bytes::lit(&[0xd5])
                                }
                            } else {
                                let a1238 = {
                                    let a1236 = v0;
                                    let a1237 = 1u32;
                                    fixed_u32::bit_and(a1236, a1237)
                                };
                                let a1239 = 0u32;
                                if equal(a1238, a1239) {
                                    Bytes::lit(&[0xd6])
                                } else {
                                    Bytes::lit(&[0xd7])
                                }
                            }
                        }
                    } else {
                        let a1242 = {
                            let a1240 = v0;
                            let a1241 = 4u32;
                            fixed_u32::bit_and(a1240, a1241)
                        };
                        let a1243 = 0u32;
                        if equal(a1242, a1243) {
                            let a1246 = {
                                let a1244 = v0;
                                let a1245 = 2u32;
                                fixed_u32::bit_and(a1244, a1245)
                            };
                            let a1247 = 0u32;
                            if equal(a1246, a1247) {
                                let a1250 = {
                                    let a1248 = v0;
                                    let a1249 = 1u32;
                                    fixed_u32::bit_and(a1248, a1249)
                                };
                                let a1251 = 0u32;
                                if equal(a1250, a1251) {
                                    Bytes::lit(&[0xd8])
                                } else {
                                    Bytes::lit(&[0xd9])
                                }
                            } else {
                                let a1254 = {
                                    let a1252 = v0;
                                    let a1253 = 1u32;
                                    fixed_u32::bit_and(a1252, a1253)
                                };
                                let a1255 = 0u32;
                                if equal(a1254, a1255) {
                                    Bytes::lit(&[0xda])
                                } else {
                                    Bytes::lit(&[0xdb])
                                }
                            }
                        } else {
                            let a1258 = {
                                let a1256 = v0;
                                let a1257 = 2u32;
                                fixed_u32::bit_and(a1256, a1257)
                            };
                            let a1259 = 0u32;
                            if equal(a1258, a1259) {
                                let a1262 = {
                                    let a1260 = v0;
                                    let a1261 = 1u32;
                                    fixed_u32::bit_and(a1260, a1261)
                                };
                                let a1263 = 0u32;
                                if equal(a1262, a1263) {
                                    Bytes::lit(&[0xdc])
                                } else {
                                    Bytes::lit(&[0xdd])
                                }
                            } else {
                                let a1266 = {
                                    let a1264 = v0;
                                    let a1265 = 1u32;
                                    fixed_u32::bit_and(a1264, a1265)
                                };
                                let a1267 = 0u32;
                                if equal(a1266, a1267) {
                                    Bytes::lit(&[0xde])
                                } else {
                                    Bytes::lit(&[0xdf])
                                }
                            }
                        }
                    }
                }
            } else {
                let a1270 = {
                    let a1268 = v0;
                    let a1269 = 16u32;
                    fixed_u32::bit_and(a1268, a1269)
                };
                let a1271 = 0u32;
                if equal(a1270, a1271) {
                    let a1274 = {
                        let a1272 = v0;
                        let a1273 = 8u32;
                        fixed_u32::bit_and(a1272, a1273)
                    };
                    let a1275 = 0u32;
                    if equal(a1274, a1275) {
                        let a1278 = {
                            let a1276 = v0;
                            let a1277 = 4u32;
                            fixed_u32::bit_and(a1276, a1277)
                        };
                        let a1279 = 0u32;
                        if equal(a1278, a1279) {
                            let a1282 = {
                                let a1280 = v0;
                                let a1281 = 2u32;
                                fixed_u32::bit_and(a1280, a1281)
                            };
                            let a1283 = 0u32;
                            if equal(a1282, a1283) {
                                let a1286 = {
                                    let a1284 = v0;
                                    let a1285 = 1u32;
                                    fixed_u32::bit_and(a1284, a1285)
                                };
                                let a1287 = 0u32;
                                if equal(a1286, a1287) {
                                    Bytes::lit(&[0xe0])
                                } else {
                                    Bytes::lit(&[0xe1])
                                }
                            } else {
                                let a1290 = {
                                    let a1288 = v0;
                                    let a1289 = 1u32;
                                    fixed_u32::bit_and(a1288, a1289)
                                };
                                let a1291 = 0u32;
                                if equal(a1290, a1291) {
                                    Bytes::lit(&[0xe2])
                                } else {
                                    Bytes::lit(&[0xe3])
                                }
                            }
                        } else {
                            let a1294 = {
                                let a1292 = v0;
                                let a1293 = 2u32;
                                fixed_u32::bit_and(a1292, a1293)
                            };
                            let a1295 = 0u32;
                            if equal(a1294, a1295) {
                                let a1298 = {
                                    let a1296 = v0;
                                    let a1297 = 1u32;
                                    fixed_u32::bit_and(a1296, a1297)
                                };
                                let a1299 = 0u32;
                                if equal(a1298, a1299) {
                                    Bytes::lit(&[0xe4])
                                } else {
                                    Bytes::lit(&[0xe5])
                                }
                            } else {
                                let a1302 = {
                                    let a1300 = v0;
                                    let a1301 = 1u32;
                                    fixed_u32::bit_and(a1300, a1301)
                                };
                                let a1303 = 0u32;
                                if equal(a1302, a1303) {
                                    Bytes::lit(&[0xe6])
                                } else {
                                    Bytes::lit(&[0xe7])
                                }
                            }
                        }
                    } else {
                        let a1306 = {
                            let a1304 = v0;
                            let a1305 = 4u32;
                            fixed_u32::bit_and(a1304, a1305)
                        };
                        let a1307 = 0u32;
                        if equal(a1306, a1307) {
                            let a1310 = {
                                let a1308 = v0;
                                let a1309 = 2u32;
                                fixed_u32::bit_and(a1308, a1309)
                            };
                            let a1311 = 0u32;
                            if equal(a1310, a1311) {
                                let a1314 = {
                                    let a1312 = v0;
                                    let a1313 = 1u32;
                                    fixed_u32::bit_and(a1312, a1313)
                                };
                                let a1315 = 0u32;
                                if equal(a1314, a1315) {
                                    Bytes::lit(&[0xe8])
                                } else {
                                    Bytes::lit(&[0xe9])
                                }
                            } else {
                                let a1318 = {
                                    let a1316 = v0;
                                    let a1317 = 1u32;
                                    fixed_u32::bit_and(a1316, a1317)
                                };
                                let a1319 = 0u32;
                                if equal(a1318, a1319) {
                                    Bytes::lit(&[0xea])
                                } else {
                                    Bytes::lit(&[0xeb])
                                }
                            }
                        } else {
                            let a1322 = {
                                let a1320 = v0;
                                let a1321 = 2u32;
                                fixed_u32::bit_and(a1320, a1321)
                            };
                            let a1323 = 0u32;
                            if equal(a1322, a1323) {
                                let a1326 = {
                                    let a1324 = v0;
                                    let a1325 = 1u32;
                                    fixed_u32::bit_and(a1324, a1325)
                                };
                                let a1327 = 0u32;
                                if equal(a1326, a1327) {
                                    Bytes::lit(&[0xec])
                                } else {
                                    Bytes::lit(&[0xed])
                                }
                            } else {
                                let a1330 = {
                                    let a1328 = v0;
                                    let a1329 = 1u32;
                                    fixed_u32::bit_and(a1328, a1329)
                                };
                                let a1331 = 0u32;
                                if equal(a1330, a1331) {
                                    Bytes::lit(&[0xee])
                                } else {
                                    Bytes::lit(&[0xef])
                                }
                            }
                        }
                    }
                } else {
                    let a1334 = {
                        let a1332 = v0;
                        let a1333 = 8u32;
                        fixed_u32::bit_and(a1332, a1333)
                    };
                    let a1335 = 0u32;
                    if equal(a1334, a1335) {
                        let a1338 = {
                            let a1336 = v0;
                            let a1337 = 4u32;
                            fixed_u32::bit_and(a1336, a1337)
                        };
                        let a1339 = 0u32;
                        if equal(a1338, a1339) {
                            let a1342 = {
                                let a1340 = v0;
                                let a1341 = 2u32;
                                fixed_u32::bit_and(a1340, a1341)
                            };
                            let a1343 = 0u32;
                            if equal(a1342, a1343) {
                                let a1346 = {
                                    let a1344 = v0;
                                    let a1345 = 1u32;
                                    fixed_u32::bit_and(a1344, a1345)
                                };
                                let a1347 = 0u32;
                                if equal(a1346, a1347) {
                                    Bytes::lit(&[0xf0])
                                } else {
                                    Bytes::lit(&[0xf1])
                                }
                            } else {
                                let a1350 = {
                                    let a1348 = v0;
                                    let a1349 = 1u32;
                                    fixed_u32::bit_and(a1348, a1349)
                                };
                                let a1351 = 0u32;
                                if equal(a1350, a1351) {
                                    Bytes::lit(&[0xf2])
                                } else {
                                    Bytes::lit(&[0xf3])
                                }
                            }
                        } else {
                            let a1354 = {
                                let a1352 = v0;
                                let a1353 = 2u32;
                                fixed_u32::bit_and(a1352, a1353)
                            };
                            let a1355 = 0u32;
                            if equal(a1354, a1355) {
                                let a1358 = {
                                    let a1356 = v0;
                                    let a1357 = 1u32;
                                    fixed_u32::bit_and(a1356, a1357)
                                };
                                let a1359 = 0u32;
                                if equal(a1358, a1359) {
                                    Bytes::lit(&[0xf4])
                                } else {
                                    Bytes::lit(&[0xf5])
                                }
                            } else {
                                let a1362 = {
                                    let a1360 = v0;
                                    let a1361 = 1u32;
                                    fixed_u32::bit_and(a1360, a1361)
                                };
                                let a1363 = 0u32;
                                if equal(a1362, a1363) {
                                    Bytes::lit(&[0xf6])
                                } else {
                                    Bytes::lit(&[0xf7])
                                }
                            }
                        }
                    } else {
                        let a1366 = {
                            let a1364 = v0;
                            let a1365 = 4u32;
                            fixed_u32::bit_and(a1364, a1365)
                        };
                        let a1367 = 0u32;
                        if equal(a1366, a1367) {
                            let a1370 = {
                                let a1368 = v0;
                                let a1369 = 2u32;
                                fixed_u32::bit_and(a1368, a1369)
                            };
                            let a1371 = 0u32;
                            if equal(a1370, a1371) {
                                let a1374 = {
                                    let a1372 = v0;
                                    let a1373 = 1u32;
                                    fixed_u32::bit_and(a1372, a1373)
                                };
                                let a1375 = 0u32;
                                if equal(a1374, a1375) {
                                    Bytes::lit(&[0xf8])
                                } else {
                                    Bytes::lit(&[0xf9])
                                }
                            } else {
                                let a1378 = {
                                    let a1376 = v0;
                                    let a1377 = 1u32;
                                    fixed_u32::bit_and(a1376, a1377)
                                };
                                let a1379 = 0u32;
                                if equal(a1378, a1379) {
                                    Bytes::lit(&[0xfa])
                                } else {
                                    Bytes::lit(&[0xfb])
                                }
                            }
                        } else {
                            let a1382 = {
                                let a1380 = v0;
                                let a1381 = 2u32;
                                fixed_u32::bit_and(a1380, a1381)
                            };
                            let a1383 = 0u32;
                            if equal(a1382, a1383) {
                                let a1386 = {
                                    let a1384 = v0;
                                    let a1385 = 1u32;
                                    fixed_u32::bit_and(a1384, a1385)
                                };
                                let a1387 = 0u32;
                                if equal(a1386, a1387) {
                                    Bytes::lit(&[0xfc])
                                } else {
                                    Bytes::lit(&[0xfd])
                                }
                            } else {
                                let a1390 = {
                                    let a1388 = v0;
                                    let a1389 = 1u32;
                                    fixed_u32::bit_and(a1388, a1389)
                                };
                                let a1391 = 0u32;
                                if equal(a1390, a1391) {
                                    Bytes::lit(&[0xfe])
                                } else {
                                    Bytes::lit(&[0xff])
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn f49(v0: u32, v1: u32) -> u32 {
    let m1394 = {
        let a1392 = v0;
        let a1393 = v1;
        fixed_u32::shift_right(a1392, a1393)
    };
    match m1394 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f50(v0: u32, v1: u32, v2: u32, v3: u32) -> u32 {
    let a1399 = v0;
    let a1400 = {
        let a1397 = f52(v1, 8u32);
        let a1398 = {
            let a1395 = f52(v2, 16u32);
            let a1396 = f52(v3, 24u32);
            fixed_u32::bit_or(a1395, a1396)
        };
        fixed_u32::bit_or(a1397, a1398)
    };
    fixed_u32::bit_or(a1399, a1400)
}

pub fn f51(v0: Bytes, v1: u64, v2: u64) -> u32 {
    let a1401 = v2;
    let a1402 = v1;
    if nat_lt(a1401, a1402) {
        let m1405 = {
            let a1403 = v0;
            let a1404 = v2;
            index_bytes(a1403, a1404)
        };
        match m1405 {
            None => {
                0u32
            }
            Some(v3) => {
                let m1407 = {
                    let a1406 = v3;
                    fixed_u32::convert(i128::from(a1406))
                };
                match m1407 {
                    None => {
                        0u32
                    }
                    Some(v4) => {
                        v4
                    }
                }
            }
        }
    } else {
        0u32
    }
}

pub fn f52(v0: u32, v1: u32) -> u32 {
    let m1410 = {
        let a1408 = v0;
        let a1409 = v1;
        fixed_u32::shift_left(a1408, a1409)
    };
    match m1410 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f53(v0: Adt3, v1: Adt3) -> Adt3 {
    let v2: Adt4 = f61(match v0.clone() {
        Adt3::C0(_, _, _, h1411, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1411
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h1412, _, _, _, _, _, _, _, _, _, _, _) => {
            h1412
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h1413, _, _, _, _, _, _) => {
            h1413
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1414, _) => {
            h1414
        }
    }, match v1.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1415, _) => {
            h1415
        }
    }, match v1 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1416) => {
            h1416
        }
    });
    Adt3::C0(match v0.clone() {
        Adt3::C0(h1417, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1417
        }
    }, match v0.clone() {
        Adt3::C0(_, h1418, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1418
        }
    }, match v0.clone() {
        Adt3::C0(_, _, h1419, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1419
        }
    }, match v2.clone() {
        Adt4::C0(h1420, _, _, _) => {
            h1420
        }
    }, match v2.clone() {
        Adt4::C0(_, h1421, _, _) => {
            h1421
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h1422, _, _, _, _, _, _, _, _, _, _) => {
            h1422
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h1423, _, _, _, _, _, _, _, _, _) => {
            h1423
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, h1424, _, _, _, _, _, _, _, _) => {
            h1424
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, h1425, _, _, _, _, _, _, _) => {
            h1425
        }
    }, match v2.clone() {
        Adt4::C0(_, _, h1426, _) => {
            h1426
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h1427, _, _, _, _, _) => {
            h1427
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h1428, _, _, _, _) => {
            h1428
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1429, _, _, _) => {
            h1429
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1430, _, _) => {
            h1430
        }
    }, match v2 {
        Adt4::C0(_, _, _, h1431) => {
            h1431
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1432) => {
            h1432
        }
    })
}

pub fn f54(v0: Adt3, v1: Adt3) -> Adt3 {
    let v2: Adt4 = f61(match v0.clone() {
        Adt3::C0(_, _, h1433, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1433
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, h1434, _, _, _, _, _, _, _, _) => {
            h1434
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, h1435, _, _, _, _, _, _, _) => {
            h1435
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1436, _, _) => {
            h1436
        }
    }, match v1.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1437, _, _, _) => {
            h1437
        }
    }, match v1 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1438, _, _) => {
            h1438
        }
    });
    Adt3::C0(match v0.clone() {
        Adt3::C0(h1439, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1439
        }
    }, match v0.clone() {
        Adt3::C0(_, h1440, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1440
        }
    }, match v2.clone() {
        Adt4::C0(h1441, _, _, _) => {
            h1441
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h1442, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1442
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h1443, _, _, _, _, _, _, _, _, _, _, _) => {
            h1443
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h1444, _, _, _, _, _, _, _, _, _, _) => {
            h1444
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h1445, _, _, _, _, _, _, _, _, _) => {
            h1445
        }
    }, match v2.clone() {
        Adt4::C0(_, h1446, _, _) => {
            h1446
        }
    }, match v2.clone() {
        Adt4::C0(_, _, h1447, _) => {
            h1447
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h1448, _, _, _, _, _, _) => {
            h1448
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h1449, _, _, _, _, _) => {
            h1449
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h1450, _, _, _, _) => {
            h1450
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1451, _, _, _) => {
            h1451
        }
    }, match v2 {
        Adt4::C0(_, _, _, h1452) => {
            h1452
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1453, _) => {
            h1453
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1454) => {
            h1454
        }
    })
}

pub fn f55(v0: Adt3, v1: Adt3) -> Adt3 {
    let v2: Adt4 = f61(match v0.clone() {
        Adt3::C0(_, h1455, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1455
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h1456, _, _, _, _, _, _, _, _, _) => {
            h1456
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h1457, _, _, _, _) => {
            h1457
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1458, _, _, _) => {
            h1458
        }
    }, match v1.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h1459, _, _, _, _, _) => {
            h1459
        }
    }, match v1 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h1460, _, _, _, _) => {
            h1460
        }
    });
    Adt3::C0(match v0.clone() {
        Adt3::C0(h1461, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1461
        }
    }, match v2.clone() {
        Adt4::C0(h1462, _, _, _) => {
            h1462
        }
    }, match v0.clone() {
        Adt3::C0(_, _, h1463, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1463
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h1464, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1464
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h1465, _, _, _, _, _, _, _, _, _, _, _) => {
            h1465
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h1466, _, _, _, _, _, _, _, _, _, _) => {
            h1466
        }
    }, match v2.clone() {
        Adt4::C0(_, h1467, _, _) => {
            h1467
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, h1468, _, _, _, _, _, _, _, _) => {
            h1468
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, h1469, _, _, _, _, _, _, _) => {
            h1469
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h1470, _, _, _, _, _, _) => {
            h1470
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h1471, _, _, _, _, _) => {
            h1471
        }
    }, match v2.clone() {
        Adt4::C0(_, _, h1472, _) => {
            h1472
        }
    }, match v2 {
        Adt4::C0(_, _, _, h1473) => {
            h1473
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1474, _, _) => {
            h1474
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1475, _) => {
            h1475
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1476) => {
            h1476
        }
    })
}

pub fn f56(v0: Adt3, v1: Adt3) -> Adt3 {
    let v2: Adt4 = f61(match v0.clone() {
        Adt3::C0(h1477, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1477
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h1478, _, _, _, _, _, _, _, _, _, _) => {
            h1478
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h1479, _, _, _, _, _) => {
            h1479
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1480) => {
            h1480
        }
    }, match v1.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, h1481, _, _, _, _, _, _, _) => {
            h1481
        }
    }, match v1 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h1482, _, _, _, _, _, _) => {
            h1482
        }
    });
    Adt3::C0(match v2.clone() {
        Adt4::C0(h1483, _, _, _) => {
            h1483
        }
    }, match v0.clone() {
        Adt3::C0(_, h1484, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1484
        }
    }, match v0.clone() {
        Adt3::C0(_, _, h1485, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1485
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h1486, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1486
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h1487, _, _, _, _, _, _, _, _, _, _, _) => {
            h1487
        }
    }, match v2.clone() {
        Adt4::C0(_, h1488, _, _) => {
            h1488
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h1489, _, _, _, _, _, _, _, _, _) => {
            h1489
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, h1490, _, _, _, _, _, _, _, _) => {
            h1490
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, h1491, _, _, _, _, _, _, _) => {
            h1491
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h1492, _, _, _, _, _, _) => {
            h1492
        }
    }, match v2.clone() {
        Adt4::C0(_, _, h1493, _) => {
            h1493
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h1494, _, _, _, _) => {
            h1494
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1495, _, _, _) => {
            h1495
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1496, _, _) => {
            h1496
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1497, _) => {
            h1497
        }
    }, match v2 {
        Adt4::C0(_, _, _, h1498) => {
            h1498
        }
    })
}

pub fn f57(v0: Adt3, v1: Adt3) -> Adt3 {
    let v2: Adt4 = f61(match v0.clone() {
        Adt3::C0(_, _, _, h1499, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1499
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, h1500, _, _, _, _, _, _, _, _) => {
            h1500
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h1501, _, _, _, _) => {
            h1501
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1502) => {
            h1502
        }
    }, match v1.clone() {
        Adt3::C0(_, _, _, _, _, _, h1503, _, _, _, _, _, _, _, _, _) => {
            h1503
        }
    }, match v1 {
        Adt3::C0(_, _, _, _, _, _, _, h1504, _, _, _, _, _, _, _, _) => {
            h1504
        }
    });
    Adt3::C0(match v0.clone() {
        Adt3::C0(h1505, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1505
        }
    }, match v0.clone() {
        Adt3::C0(_, h1506, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1506
        }
    }, match v0.clone() {
        Adt3::C0(_, _, h1507, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1507
        }
    }, match v2.clone() {
        Adt4::C0(h1508, _, _, _) => {
            h1508
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h1509, _, _, _, _, _, _, _, _, _, _, _) => {
            h1509
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h1510, _, _, _, _, _, _, _, _, _, _) => {
            h1510
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h1511, _, _, _, _, _, _, _, _, _) => {
            h1511
        }
    }, match v2.clone() {
        Adt4::C0(_, h1512, _, _) => {
            h1512
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, h1513, _, _, _, _, _, _, _) => {
            h1513
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h1514, _, _, _, _, _, _) => {
            h1514
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h1515, _, _, _, _, _) => {
            h1515
        }
    }, match v2.clone() {
        Adt4::C0(_, _, h1516, _) => {
            h1516
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1517, _, _, _) => {
            h1517
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1518, _, _) => {
            h1518
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1519, _) => {
            h1519
        }
    }, match v2 {
        Adt4::C0(_, _, _, h1520) => {
            h1520
        }
    })
}

pub fn f58(v0: Adt3, v1: Adt3) -> Adt3 {
    let v2: Adt4 = f61(match v0.clone() {
        Adt3::C0(_, _, h1521, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1521
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h1522, _, _, _, _, _, _, _, _, _) => {
            h1522
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h1523, _, _, _, _, _) => {
            h1523
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1524, _) => {
            h1524
        }
    }, match v1.clone() {
        Adt3::C0(_, _, _, _, h1525, _, _, _, _, _, _, _, _, _, _, _) => {
            h1525
        }
    }, match v1 {
        Adt3::C0(_, _, _, _, _, h1526, _, _, _, _, _, _, _, _, _, _) => {
            h1526
        }
    });
    Adt3::C0(match v0.clone() {
        Adt3::C0(h1527, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1527
        }
    }, match v0.clone() {
        Adt3::C0(_, h1528, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1528
        }
    }, match v2.clone() {
        Adt4::C0(h1529, _, _, _) => {
            h1529
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h1530, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1530
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h1531, _, _, _, _, _, _, _, _, _, _, _) => {
            h1531
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h1532, _, _, _, _, _, _, _, _, _, _) => {
            h1532
        }
    }, match v2.clone() {
        Adt4::C0(_, h1533, _, _) => {
            h1533
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, h1534, _, _, _, _, _, _, _, _) => {
            h1534
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, h1535, _, _, _, _, _, _, _) => {
            h1535
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h1536, _, _, _, _, _, _) => {
            h1536
        }
    }, match v2.clone() {
        Adt4::C0(_, _, h1537, _) => {
            h1537
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h1538, _, _, _, _) => {
            h1538
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1539, _, _, _) => {
            h1539
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1540, _, _) => {
            h1540
        }
    }, match v2 {
        Adt4::C0(_, _, _, h1541) => {
            h1541
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1542) => {
            h1542
        }
    })
}

pub fn f59(v0: Adt3, v1: Adt3) -> Adt3 {
    let v2: Adt4 = f61(match v0.clone() {
        Adt3::C0(_, h1543, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1543
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h1544, _, _, _, _, _, _, _, _, _, _) => {
            h1544
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h1545, _, _, _, _, _, _) => {
            h1545
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1546, _, _) => {
            h1546
        }
    }, match v1.clone() {
        Adt3::C0(_, _, h1547, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1547
        }
    }, match v1 {
        Adt3::C0(_, _, _, h1548, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1548
        }
    });
    Adt3::C0(match v0.clone() {
        Adt3::C0(h1549, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1549
        }
    }, match v2.clone() {
        Adt4::C0(h1550, _, _, _) => {
            h1550
        }
    }, match v0.clone() {
        Adt3::C0(_, _, h1551, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1551
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h1552, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1552
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h1553, _, _, _, _, _, _, _, _, _, _, _) => {
            h1553
        }
    }, match v2.clone() {
        Adt4::C0(_, h1554, _, _) => {
            h1554
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h1555, _, _, _, _, _, _, _, _, _) => {
            h1555
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, h1556, _, _, _, _, _, _, _, _) => {
            h1556
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, h1557, _, _, _, _, _, _, _) => {
            h1557
        }
    }, match v2.clone() {
        Adt4::C0(_, _, h1558, _) => {
            h1558
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h1559, _, _, _, _, _) => {
            h1559
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h1560, _, _, _, _) => {
            h1560
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1561, _, _, _) => {
            h1561
        }
    }, match v2 {
        Adt4::C0(_, _, _, h1562) => {
            h1562
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1563, _) => {
            h1563
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1564) => {
            h1564
        }
    })
}

pub fn f60(v0: Adt3, v1: Adt3) -> Adt3 {
    let v2: Adt4 = f61(match v0.clone() {
        Adt3::C0(h1565, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1565
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h1566, _, _, _, _, _, _, _, _, _, _, _) => {
            h1566
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, h1567, _, _, _, _, _, _, _) => {
            h1567
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1568, _, _, _) => {
            h1568
        }
    }, match v1.clone() {
        Adt3::C0(h1569, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1569
        }
    }, match v1 {
        Adt3::C0(_, h1570, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1570
        }
    });
    Adt3::C0(match v2.clone() {
        Adt4::C0(h1571, _, _, _) => {
            h1571
        }
    }, match v0.clone() {
        Adt3::C0(_, h1572, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1572
        }
    }, match v0.clone() {
        Adt3::C0(_, _, h1573, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1573
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h1574, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1574
        }
    }, match v2.clone() {
        Adt4::C0(_, h1575, _, _) => {
            h1575
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h1576, _, _, _, _, _, _, _, _, _, _) => {
            h1576
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h1577, _, _, _, _, _, _, _, _, _) => {
            h1577
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, h1578, _, _, _, _, _, _, _, _) => {
            h1578
        }
    }, match v2.clone() {
        Adt4::C0(_, _, h1579, _) => {
            h1579
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h1580, _, _, _, _, _, _) => {
            h1580
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h1581, _, _, _, _, _) => {
            h1581
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h1582, _, _, _, _) => {
            h1582
        }
    }, match v2 {
        Adt4::C0(_, _, _, h1583) => {
            h1583
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1584, _, _) => {
            h1584
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1585, _) => {
            h1585
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1586) => {
            h1586
        }
    })
}

pub fn f61(v0: u32, v1: u32, v2: u32, v3: u32, v4: u32, v5: u32) -> Adt4 {
    let v6: u32 = f62(f62(v0, v1), v4);
    let v7: u32 = f63(f38(v3, v6), 16u32, 16u32);
    let v8: u32 = f62(v2, v7);
    let v9: u32 = f63(f38(v1, v8), 12u32, 20u32);
    let v10: u32 = f62(f62(v6, v9), v5);
    let v11: u32 = f63(f38(v7, v10), 8u32, 24u32);
    let v12: u32 = f62(v8, v11);
    let v13: u32 = f63(f38(v9, v12), 7u32, 25u32);
    Adt4::C0(v10, v13, v12, v11)
}

pub fn f62(v0: u32, v1: u32) -> u32 {
    let m1589 = {
        let a1587 = f64(v0);
        let a1588 = f64(v1);
        fixed_u64::checked_add(a1587, a1588)
    };
    match m1589 {
        None => {
            0u32
        }
        Some(v2) => {
            f65(v2)
        }
    }
}

pub fn f63(v0: u32, v1: u32, v2: u32) -> u32 {
    let a1590 = f49(v0, v1);
    let a1591 = f52(v0, v2);
    fixed_u32::bit_or(a1590, a1591)
}

pub fn f64(v0: u32) -> u64 {
    let m1593 = {
        let a1592 = v0;
        fixed_u64::convert(i128::from(a1592))
    };
    match m1593 {
        None => {
            0u64
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f65(v0: u64) -> u32 {
    let m1597 = {
        let a1596 = {
            let a1594 = v0;
            let a1595 = 4294967295u64;
            fixed_u64::bit_and(a1594, a1595)
        };
        fixed_u32::convert(i128::from(a1596))
    };
    match m1597 {
        None => {
            0u32
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn run(p0: Bytes, p1: Bytes, p2: u64) -> R<Bytes> {
    f0(p0, p1, p2)
}
