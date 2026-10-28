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
    F8(u32, Bytes, Adt1, Adt0),
}

impl Fn0 {
    pub fn apply(&self, p0: (u64, List<Adt1>)) -> R<(u64, List<Adt1>)> {
        match self {
            Fn0::F8(k0, k1, k2, k3) => f8(*k0, k1.clone(), k2.clone(), k3.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn1 {
    F10(Adt2),
}

impl Fn1 {
    pub fn apply(&self, p0: (u64, Bytes)) -> R<(u64, Bytes)> {
        match self {
            Fn1::F10(k0) => f10(k0.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn2 {
    F12(u32, Adt1),
}

impl Fn2 {
    pub fn apply(&self, p0: Adt2, p1: Adt1) -> Adt2 {
        match self {
            Fn2::F12(k0, k1) => f12(*k0, k1.clone(), p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn3 {
    F23(u32, u64, u32, u64, u32, Bytes),
}

impl Fn3 {
    pub fn apply(&self, p0: (u64, Adt1)) -> R<(u64, Adt1)> {
        match self {
            Fn3::F23(k0, k1, k2, k3, k4, k5) => f23(*k0, *k1, *k2, *k3, *k4, k5.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn4 {
    F29(u32, Adt1),
}

impl Fn4 {
    pub fn apply(&self, p0: (Adt1, (u64, List<Adt1>))) -> Option<(Adt1, (u64, List<Adt1>))> {
        match self {
            Fn4::F29(k0, k1) => f29(*k0, k1.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn5 {
    F37,
}

impl Fn5 {
    pub fn apply(&self, p0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
        match self {
            Fn5::F37 => f37(p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn6 {
    F41,
}

impl Fn6 {
    pub fn apply(&self, p0: (Adt3, Adt3)) -> (Adt3, Adt3) {
        match self {
            Fn6::F41 => f41(p0),
        }
    }
}

pub fn f0(v0: Bytes, v1: u64) -> R<Bytes> {
    f1(f2(f3(f4(), 0u32), v0)?, v1)
}

pub fn f1(v0: Adt0, v1: u64) -> R<Bytes> {
    f5(f6(v0)?, v1)
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
    let v7: (u64, List<Adt1>) = f7(Fn0::F8(v5, v2.clone(), v6.clone(), v0.clone()), v4, (0u64, match v0.clone() {
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

pub fn f4() -> Adt1 {
    Adt1::C0(1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, 1359893119u32, 2600822924u32, 528734635u32, 1541459225u32)
}

pub fn f5(v0: Adt2, v1: u64) -> R<Bytes> {
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
    let v3: (u64, Bytes) = f9(Fn1::F10(v0), v2, (0u64, Bytes::lit(&[])))?;
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

pub fn f6(v0: Adt0) -> R<Adt2> {
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
    Ok(f11(Fn2::F12(v2, v1.clone()), f13(v1, v2, v3.clone(), 0u64, {
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

pub fn f7(v0: Fn0, p47: u64, p48: (u64, List<Adt1>)) -> R<(u64, List<Adt1>)> {
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

pub fn f8(v0: u32, v1: Bytes, v2: Adt1, v3: Adt0, v4: (u64, List<Adt1>)) -> R<(u64, List<Adt1>)> {
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
    }, f14(v2.clone(), v0, {
        let (_, h56) = v4;
        h56
    }, f15(f13(v2, v0, v1, {
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

pub fn f9(v0: Fn1, p65: u64, p66: (u64, Bytes)) -> R<(u64, Bytes)> {
    let m64 = (p65, p66);
    {
        let mut m64 = m64;
        loop {
            let step = {
                let (v1, v2) = m64;
                let m62 = v1;
                if m62 == 0 {
                    Err::<(u64, (u64, Bytes)), R<(u64, Bytes)>>(Ok(v2))
                } else {
                    let v3 = m62 - 1;
                    Ok::<(u64, (u64, Bytes)), R<(u64, Bytes)>>((v3, {
                        let c63: Fn1 = v0.clone();
                        c63.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m64 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f10(v0: Adt2, v1: (u64, Bytes)) -> R<(u64, Bytes)> {
    Ok(({
        let a68 = {
            let (h67, _) = v1.clone();
            h67
        };
        let a69 = 1u64;
        nat_add(a68, a69)?
    }, {
        let a72 = {
            let (_, h70) = v1;
            h70
        };
        let a73 = f16(f17(v0, {
            let (h71, _) = v1;
            h71
        }));
        append_bytes(a72, a73)
    }))
}

pub fn f11(v0: Fn2, p77: Adt2, p78: List<Adt1>) -> Adt2 {
    let m76 = (p77, p78);
    {
        let mut m76 = m76;
        loop {
            let step = {
                let (v1, v2) = m76;
                let m74 = v2;
                match m74.uncons() {
                    None => {
                        Err::<(Adt2, List<Adt1>), Adt2>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt2, List<Adt1>), Adt2>(({
                            let c75: Fn2 = v0.clone();
                            c75.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m76 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f12(v0: u32, v1: Adt1, v2: Adt2, v3: Adt1) -> Adt2 {
    f18(v3, f15(v2), v1, v0)
}

pub fn f13(v0: Adt1, v1: u32, v2: Bytes, v3: u64, v4: u64, v5: u64) -> R<Adt2> {
    let v6: u64 = {
        let a79 = v3;
        let a80 = v4;
        nat_add(a79, a80)?
    };
    let v7: u64 = f19(v4)?;
    let v8: u32 = f20(v5);
    let v9: u32 = f21(v5);
    let v10: (u64, Adt1) = f22(Fn3::F23(v1, v3, v9, v6, v8, v2.clone()), {
        let a81 = v7;
        let a82 = 64u64;
        let a83 = 0u64;
        nat_quot(a81, a82, a83)
    }, (0u64, v0))?;
    Ok(Adt2::C0({
        let (_, h84) = v10;
        h84
    }, f24(v2, v6, {
        let a85 = v3;
        let a86 = v7;
        nat_add(a85, a86)?
    })?, v5, f25({
        let a87 = v4;
        let a88 = v7;
        nat_sub(a87, a88)
    }, 7u64), {
        let a91 = v1;
        let a92 = {
            let a89 = f26();
            let a90 = f27(v7);
            fixed_u32::bit_or(a89, a90)
        };
        fixed_u32::bit_or(a91, a92)
    }))
}

pub fn f14(v0: Adt1, v1: u32, v2: List<Adt1>, v3: Adt1, v4: u64) -> List<Adt1> {
    let v5: (Adt1, (u64, List<Adt1>)) = {
        let (h93, _) = f28(Fn4::F29(v1, v0), 64u64, (v3, (v4, v2)));
        h93
    };
    List::cons({
        let (h94, _) = v5;
        h94
    }, {
        let (_, h95) = {
            let (_, h96) = v5;
            h96
        };
        h95
    })
}

pub fn f15(v0: Adt2) -> Adt1 {
    f30(f31(match v0.clone() {
        Adt2::C0(h97, _, _, _, _) => {
            h97
        }
    }, match v0.clone() {
        Adt2::C0(_, h98, _, _, _) => {
            h98
        }
    }, f20(match v0.clone() {
        Adt2::C0(_, _, h99, _, _) => {
            h99
        }
    }), f21(match v0.clone() {
        Adt2::C0(_, _, h100, _, _) => {
            h100
        }
    }), match v0.clone() {
        Adt2::C0(_, _, _, h101, _) => {
            h101
        }
    }, match v0 {
        Adt2::C0(_, _, _, _, h102) => {
            h102
        }
    }))
}

pub fn f16(v0: Adt3) -> Bytes {
    let a147 = f32(match v0.clone() {
        Adt3::C0(h103, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h103
        }
    });
    let a148 = {
        let a145 = f32(match v0.clone() {
            Adt3::C0(_, h104, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                h104
            }
        });
        let a146 = {
            let a143 = f32(match v0.clone() {
                Adt3::C0(_, _, h105, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                    h105
                }
            });
            let a144 = {
                let a141 = f32(match v0.clone() {
                    Adt3::C0(_, _, _, h106, _, _, _, _, _, _, _, _, _, _, _, _) => {
                        h106
                    }
                });
                let a142 = {
                    let a139 = f32(match v0.clone() {
                        Adt3::C0(_, _, _, _, h107, _, _, _, _, _, _, _, _, _, _, _) => {
                            h107
                        }
                    });
                    let a140 = {
                        let a137 = f32(match v0.clone() {
                            Adt3::C0(_, _, _, _, _, h108, _, _, _, _, _, _, _, _, _, _) => {
                                h108
                            }
                        });
                        let a138 = {
                            let a135 = f32(match v0.clone() {
                                Adt3::C0(_, _, _, _, _, _, h109, _, _, _, _, _, _, _, _, _) => {
                                    h109
                                }
                            });
                            let a136 = {
                                let a133 = f32(match v0.clone() {
                                    Adt3::C0(_, _, _, _, _, _, _, h110, _, _, _, _, _, _, _, _) => {
                                        h110
                                    }
                                });
                                let a134 = {
                                    let a131 = f32(match v0.clone() {
                                        Adt3::C0(_, _, _, _, _, _, _, _, h111, _, _, _, _, _, _, _) => {
                                            h111
                                        }
                                    });
                                    let a132 = {
                                        let a129 = f32(match v0.clone() {
                                            Adt3::C0(_, _, _, _, _, _, _, _, _, h112, _, _, _, _, _, _) => {
                                                h112
                                            }
                                        });
                                        let a130 = {
                                            let a127 = f32(match v0.clone() {
                                                Adt3::C0(_, _, _, _, _, _, _, _, _, _, h113, _, _, _, _, _) => {
                                                    h113
                                                }
                                            });
                                            let a128 = {
                                                let a125 = f32(match v0.clone() {
                                                    Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h114, _, _, _, _) => {
                                                        h114
                                                    }
                                                });
                                                let a126 = {
                                                    let a123 = f32(match v0.clone() {
                                                        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h115, _, _, _) => {
                                                            h115
                                                        }
                                                    });
                                                    let a124 = {
                                                        let a121 = f32(match v0.clone() {
                                                            Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h116, _, _) => {
                                                                h116
                                                            }
                                                        });
                                                        let a122 = {
                                                            let a119 = f32(match v0.clone() {
                                                                Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h117, _) => {
                                                                    h117
                                                                }
                                                            });
                                                            let a120 = f32(match v0 {
                                                                Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h118) => {
                                                                    h118
                                                                }
                                                            });
                                                            append_bytes(a119, a120)
                                                        };
                                                        append_bytes(a121, a122)
                                                    };
                                                    append_bytes(a123, a124)
                                                };
                                                append_bytes(a125, a126)
                                            };
                                            append_bytes(a127, a128)
                                        };
                                        append_bytes(a129, a130)
                                    };
                                    append_bytes(a131, a132)
                                };
                                append_bytes(a133, a134)
                            };
                            append_bytes(a135, a136)
                        };
                        append_bytes(a137, a138)
                    };
                    append_bytes(a139, a140)
                };
                append_bytes(a141, a142)
            };
            append_bytes(a143, a144)
        };
        append_bytes(a145, a146)
    };
    append_bytes(a147, a148)
}

pub fn f17(v0: Adt2, v1: u64) -> Adt3 {
    f31(match v0.clone() {
        Adt2::C0(h149, _, _, _, _) => {
            h149
        }
    }, match v0.clone() {
        Adt2::C0(_, h150, _, _, _) => {
            h150
        }
    }, f20(v1), f21(v1), match v0.clone() {
        Adt2::C0(_, _, _, h151, _) => {
            h151
        }
    }, {
        let a153 = match v0 {
            Adt2::C0(_, _, _, _, h152) => {
                h152
            }
        };
        let a154 = f33();
        fixed_u32::bit_or(a153, a154)
    })
}

pub fn f18(v0: Adt1, v1: Adt1, v2: Adt1, v3: u32) -> Adt2 {
    Adt2::C0(v2, Adt3::C0(match v0.clone() {
        Adt1::C0(h155, _, _, _, _, _, _, _) => {
            h155
        }
    }, match v0.clone() {
        Adt1::C0(_, h156, _, _, _, _, _, _) => {
            h156
        }
    }, match v0.clone() {
        Adt1::C0(_, _, h157, _, _, _, _, _) => {
            h157
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, h158, _, _, _, _) => {
            h158
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, h159, _, _, _) => {
            h159
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, _, h160, _, _) => {
            h160
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, _, _, h161, _) => {
            h161
        }
    }, match v0 {
        Adt1::C0(_, _, _, _, _, _, _, h162) => {
            h162
        }
    }, match v1.clone() {
        Adt1::C0(h163, _, _, _, _, _, _, _) => {
            h163
        }
    }, match v1.clone() {
        Adt1::C0(_, h164, _, _, _, _, _, _) => {
            h164
        }
    }, match v1.clone() {
        Adt1::C0(_, _, h165, _, _, _, _, _) => {
            h165
        }
    }, match v1.clone() {
        Adt1::C0(_, _, _, h166, _, _, _, _) => {
            h166
        }
    }, match v1.clone() {
        Adt1::C0(_, _, _, _, h167, _, _, _) => {
            h167
        }
    }, match v1.clone() {
        Adt1::C0(_, _, _, _, _, h168, _, _) => {
            h168
        }
    }, match v1.clone() {
        Adt1::C0(_, _, _, _, _, _, h169, _) => {
            h169
        }
    }, match v1 {
        Adt1::C0(_, _, _, _, _, _, _, h170) => {
            h170
        }
    }), 0u64, 64u32, {
        let a171 = v3;
        let a172 = f34();
        fixed_u32::bit_or(a171, a172)
    })
}

pub fn f19(v0: u64) -> R<u64> {
    let a173 = v0;
    let a174 = 0u64;
    if nat_eq(a173, a174) {
        Ok(0u64)
    } else {
        let a180 = {
            let a177 = {
                let a175 = v0;
                let a176 = 1u64;
                nat_sub(a175, a176)
            };
            let a178 = 64u64;
            let a179 = 0u64;
            nat_quot(a177, a178, a179)
        };
        let a181 = 64u64;
        nat_mul(a180, a181)
    }
}

pub fn f20(v0: u64) -> u32 {
    f25({
        let a182 = v0;
        let a183 = 4294967296u64;
        let a184 = 0u64;
        nat_rem(a182, a183, a184)
    }, 32u64)
}

pub fn f21(v0: u64) -> u32 {
    let a185 = v0;
    let a186 = 4294967296u64;
    if nat_lt(a185, a186) {
        0u32
    } else {
        f25({
            let a187 = v0;
            let a188 = 4294967296u64;
            let a189 = 0u64;
            nat_quot(a187, a188, a189)
        }, 32u64)
    }
}

pub fn f22(v0: Fn3, p193: u64, p194: (u64, Adt1)) -> R<(u64, Adt1)> {
    let m192 = (p193, p194);
    {
        let mut m192 = m192;
        loop {
            let step = {
                let (v1, v2) = m192;
                let m190 = v1;
                if m190 == 0 {
                    Err::<(u64, (u64, Adt1)), R<(u64, Adt1)>>(Ok(v2))
                } else {
                    let v3 = m190 - 1;
                    Ok::<(u64, (u64, Adt1)), R<(u64, Adt1)>>((v3, {
                        let c191: Fn3 = v0.clone();
                        c191.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m192 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f23(v0: u32, v1: u64, v2: u32, v3: u64, v4: u32, v5: Bytes, v6: (u64, Adt1)) -> R<(u64, Adt1)> {
    let v7: u64 = {
        let (h195, _) = v6;
        h195
    };
    Ok(({
        let a196 = v7;
        let a197 = 64u64;
        nat_add(a196, a197)?
    }, f30(f31({
        let (_, h198) = v6;
        h198
    }, f24(v5, v3, {
        let a199 = v1;
        let a200 = v7;
        nat_add(a199, a200)?
    })?, v4, v2, 64u32, {
        let a201 = v0;
        let a202 = f27(v7);
        fixed_u32::bit_or(a201, a202)
    }))))
}

pub fn f24(v0: Bytes, v1: u64, v2: u64) -> R<Adt3> {
    Ok(Adt3::C0(f35(v0.clone(), v1, {
        let a203 = v2;
        let a204 = 0u64;
        nat_add(a203, a204)?
    })?, f35(v0.clone(), v1, {
        let a205 = v2;
        let a206 = 4u64;
        nat_add(a205, a206)?
    })?, f35(v0.clone(), v1, {
        let a207 = v2;
        let a208 = 8u64;
        nat_add(a207, a208)?
    })?, f35(v0.clone(), v1, {
        let a209 = v2;
        let a210 = 12u64;
        nat_add(a209, a210)?
    })?, f35(v0.clone(), v1, {
        let a211 = v2;
        let a212 = 16u64;
        nat_add(a211, a212)?
    })?, f35(v0.clone(), v1, {
        let a213 = v2;
        let a214 = 20u64;
        nat_add(a213, a214)?
    })?, f35(v0.clone(), v1, {
        let a215 = v2;
        let a216 = 24u64;
        nat_add(a215, a216)?
    })?, f35(v0.clone(), v1, {
        let a217 = v2;
        let a218 = 28u64;
        nat_add(a217, a218)?
    })?, f35(v0.clone(), v1, {
        let a219 = v2;
        let a220 = 32u64;
        nat_add(a219, a220)?
    })?, f35(v0.clone(), v1, {
        let a221 = v2;
        let a222 = 36u64;
        nat_add(a221, a222)?
    })?, f35(v0.clone(), v1, {
        let a223 = v2;
        let a224 = 40u64;
        nat_add(a223, a224)?
    })?, f35(v0.clone(), v1, {
        let a225 = v2;
        let a226 = 44u64;
        nat_add(a225, a226)?
    })?, f35(v0.clone(), v1, {
        let a227 = v2;
        let a228 = 48u64;
        nat_add(a227, a228)?
    })?, f35(v0.clone(), v1, {
        let a229 = v2;
        let a230 = 52u64;
        nat_add(a229, a230)?
    })?, f35(v0.clone(), v1, {
        let a231 = v2;
        let a232 = 56u64;
        nat_add(a231, a232)?
    })?, f35(v0, v1, {
        let a233 = v2;
        let a234 = 60u64;
        nat_add(a233, a234)?
    })?))
}

pub fn f25(v0: u64, v1: u64) -> u32 {
    let (_, h235) = {
        let (_, h236) = f36(Fn5::F37, v1, (v0, (1u32, 0u32)));
        h236
    };
    h235
}

pub fn f26() -> u32 {
    2u32
}

pub fn f27(v0: u64) -> u32 {
    let a237 = v0;
    let a238 = 0u64;
    if nat_eq(a237, a238) {
        f38()
    } else {
        0u32
    }
}

pub fn f28(v0: Fn4, p243: u64, p244: (Adt1, (u64, List<Adt1>))) -> ((Adt1, (u64, List<Adt1>)), bool) {
    let m242 = (p243, p244);
    {
        let mut m242 = m242;
        loop {
            let step = {
                let (v1, v2) = m242;
                let m239 = v1;
                if m239 == 0 {
                    Err::<(u64, (Adt1, (u64, List<Adt1>))), ((Adt1, (u64, List<Adt1>)), bool)>((v2, false))
                } else {
                    let v3 = m239 - 1;
                    let m241 = {
                        let c240: Fn4 = v0.clone();
                        c240.apply(v2.clone())
                    };
                    match m241 {
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
                Ok(next) => { m242 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f29(v0: u32, v1: Adt1, v2: (Adt1, (u64, List<Adt1>))) -> Option<(Adt1, (u64, List<Adt1>))> {
    f39(v1, v0, v2)
}

pub fn f30(v0: Adt3) -> Adt1 {
    Adt1::C0(match v0.clone() {
        Adt3::C0(h245, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h245
        }
    }, match v0.clone() {
        Adt3::C0(_, h246, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h246
        }
    }, match v0.clone() {
        Adt3::C0(_, _, h247, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h247
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h248, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h248
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h249, _, _, _, _, _, _, _, _, _, _, _) => {
            h249
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h250, _, _, _, _, _, _, _, _, _, _) => {
            h250
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h251, _, _, _, _, _, _, _, _, _) => {
            h251
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, h252, _, _, _, _, _, _, _, _) => {
            h252
        }
    })
}

pub fn f31(v0: Adt1, v1: Adt3, v2: u32, v3: u32, v4: u32, v5: u32) -> Adt3 {
    let v6: Adt3 = {
        let (h253, _) = f40(Fn6::F41, 7u64, (Adt3::C0(match v0.clone() {
            Adt1::C0(h254, _, _, _, _, _, _, _) => {
                h254
            }
        }, match v0.clone() {
            Adt1::C0(_, h255, _, _, _, _, _, _) => {
                h255
            }
        }, match v0.clone() {
            Adt1::C0(_, _, h256, _, _, _, _, _) => {
                h256
            }
        }, match v0.clone() {
            Adt1::C0(_, _, _, h257, _, _, _, _) => {
                h257
            }
        }, match v0.clone() {
            Adt1::C0(_, _, _, _, h258, _, _, _) => {
                h258
            }
        }, match v0.clone() {
            Adt1::C0(_, _, _, _, _, h259, _, _) => {
                h259
            }
        }, match v0.clone() {
            Adt1::C0(_, _, _, _, _, _, h260, _) => {
                h260
            }
        }, match v0.clone() {
            Adt1::C0(_, _, _, _, _, _, _, h261) => {
                h261
            }
        }, 1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, v2, v3, v4, v5), v1));
        h253
    };
    Adt3::C0(f42(match v6.clone() {
        Adt3::C0(h262, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h262
        }
    }, match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, h263, _, _, _, _, _, _, _) => {
            h263
        }
    }), f42(match v6.clone() {
        Adt3::C0(_, h264, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h264
        }
    }, match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h265, _, _, _, _, _, _) => {
            h265
        }
    }), f42(match v6.clone() {
        Adt3::C0(_, _, h266, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h266
        }
    }, match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h267, _, _, _, _, _) => {
            h267
        }
    }), f42(match v6.clone() {
        Adt3::C0(_, _, _, h268, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h268
        }
    }, match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h269, _, _, _, _) => {
            h269
        }
    }), f42(match v6.clone() {
        Adt3::C0(_, _, _, _, h270, _, _, _, _, _, _, _, _, _, _, _) => {
            h270
        }
    }, match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h271, _, _, _) => {
            h271
        }
    }), f42(match v6.clone() {
        Adt3::C0(_, _, _, _, _, h272, _, _, _, _, _, _, _, _, _, _) => {
            h272
        }
    }, match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h273, _, _) => {
            h273
        }
    }), f42(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, h274, _, _, _, _, _, _, _, _, _) => {
            h274
        }
    }, match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h275, _) => {
            h275
        }
    }), f42(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, h276, _, _, _, _, _, _, _, _) => {
            h276
        }
    }, match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h277) => {
            h277
        }
    }), f42(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, h278, _, _, _, _, _, _, _) => {
            h278
        }
    }, match v0.clone() {
        Adt1::C0(h279, _, _, _, _, _, _, _) => {
            h279
        }
    }), f42(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h280, _, _, _, _, _, _) => {
            h280
        }
    }, match v0.clone() {
        Adt1::C0(_, h281, _, _, _, _, _, _) => {
            h281
        }
    }), f42(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h282, _, _, _, _, _) => {
            h282
        }
    }, match v0.clone() {
        Adt1::C0(_, _, h283, _, _, _, _, _) => {
            h283
        }
    }), f42(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h284, _, _, _, _) => {
            h284
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, h285, _, _, _, _) => {
            h285
        }
    }), f42(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h286, _, _, _) => {
            h286
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, h287, _, _, _) => {
            h287
        }
    }), f42(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h288, _, _) => {
            h288
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, _, h289, _, _) => {
            h289
        }
    }), f42(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h290, _) => {
            h290
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, _, _, h291, _) => {
            h291
        }
    }), f42(match v6 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h292) => {
            h292
        }
    }, match v0 {
        Adt1::C0(_, _, _, _, _, _, _, h293) => {
            h293
        }
    }))
}

pub fn f32(v0: u32) -> Bytes {
    let a304 = f43({
        let a294 = v0;
        let a295 = 255u32;
        fixed_u32::bit_and(a294, a295)
    });
    let a305 = {
        let a302 = f43({
            let a296 = f44(v0, 8u32);
            let a297 = 255u32;
            fixed_u32::bit_and(a296, a297)
        });
        let a303 = {
            let a300 = f43({
                let a298 = f44(v0, 16u32);
                let a299 = 255u32;
                fixed_u32::bit_and(a298, a299)
            });
            let a301 = f43(f44(v0, 24u32));
            append_bytes(a300, a301)
        };
        append_bytes(a302, a303)
    };
    append_bytes(a304, a305)
}

pub fn f33() -> u32 {
    8u32
}

pub fn f34() -> u32 {
    4u32
}

pub fn f35(v0: Bytes, v1: u64, v2: u64) -> R<u32> {
    Ok(f45(f46(v0.clone(), v1, v2), f46(v0.clone(), v1, {
        let a306 = v2;
        let a307 = 1u64;
        nat_add(a306, a307)?
    }), f46(v0.clone(), v1, {
        let a308 = v2;
        let a309 = 2u64;
        nat_add(a308, a309)?
    }), f46(v0, v1, {
        let a310 = v2;
        let a311 = 3u64;
        nat_add(a310, a311)?
    })))
}

pub fn f36(v0: Fn5, p315: u64, p316: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let m314 = (p315, p316);
    {
        let mut m314 = m314;
        loop {
            let step = {
                let (v1, v2) = m314;
                let m312 = v1;
                if m312 == 0 {
                    Err::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>(v2)
                } else {
                    let v3 = m312 - 1;
                    Ok::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>((v3, {
                        let c313: Fn5 = v0.clone();
                        c313.apply(v2)
                    }))
                }
            };
            match step {
                Ok(next) => { m314 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f37(v0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let v1: u64 = {
        let (h317, _) = v0;
        h317
    };
    let v2: u32 = {
        let (h318, _) = {
            let (_, h319) = v0;
            h319
        };
        h318
    };
    let v3: u32 = {
        let (_, h320) = {
            let (_, h321) = v0;
            h321
        };
        h320
    };
    ({
        let a322 = v1;
        let a323 = 2u64;
        let a324 = 0u64;
        nat_quot(a322, a323, a324)
    }, (f47(v2, 1u32), {
        let a328 = {
            let a325 = v1;
            let a326 = 2u64;
            let a327 = 0u64;
            nat_rem(a325, a326, a327)
        };
        let a329 = 1u64;
        if nat_eq(a328, a329) {
            let a330 = v3;
            let a331 = v2;
            fixed_u32::bit_or(a330, a331)
        } else {
            v3
        }
    }))
}

pub fn f38() -> u32 {
    1u32
}

pub fn f39(v0: Adt1, v1: u32, v2: (Adt1, (u64, List<Adt1>))) -> Option<(Adt1, (u64, List<Adt1>))> {
    let v3: u64 = {
        let (h332, _) = {
            let (_, h333) = v2.clone();
            h333
        };
        h332
    };
    let a337 = {
        let a334 = v3;
        let a335 = 2u64;
        let a336 = 0u64;
        nat_rem(a334, a335, a336)
    };
    let a338 = 0u64;
    if nat_eq(a337, a338) {
        let m341 = {
            let (_, h339) = {
                let (_, h340) = v2;
                h340
            };
            h339
        };
        match m341.uncons() {
            None => {
                None::<(Adt1, (u64, List<Adt1>))>
            }
            Some((v4, v5)) => {
                Some((f15(f18(v4, {
                    let (h342, _) = v2;
                    h342
                }, v0, v1)), ({
                    let a343 = v3;
                    let a344 = 2u64;
                    let a345 = 0u64;
                    nat_quot(a343, a344, a345)
                }, v5)))
            }
        }
    } else {
        None::<(Adt1, (u64, List<Adt1>))>
    }
}

pub fn f40(v0: Fn6, p349: u64, p350: (Adt3, Adt3)) -> (Adt3, Adt3) {
    let m348 = (p349, p350);
    {
        let mut m348 = m348;
        loop {
            let step = {
                let (v1, v2) = m348;
                let m346 = v1;
                if m346 == 0 {
                    Err::<(u64, (Adt3, Adt3)), (Adt3, Adt3)>(v2)
                } else {
                    let v3 = m346 - 1;
                    Ok::<(u64, (Adt3, Adt3)), (Adt3, Adt3)>((v3, {
                        let c347: Fn6 = v0.clone();
                        c347.apply(v2)
                    }))
                }
            };
            match step {
                Ok(next) => { m348 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f41(v0: (Adt3, Adt3)) -> (Adt3, Adt3) {
    (f48({
        let (h351, _) = v0.clone();
        h351
    }, {
        let (_, h352) = v0.clone();
        h352
    }), f49({
        let (_, h353) = v0;
        h353
    }))
}

pub fn f42(v0: u32, v1: u32) -> u32 {
    let a354 = v0;
    let a355 = v1;
    fixed_u32::bit_xor(a354, a355)
}

pub fn f43(v0: u32) -> Bytes {
    let a358 = {
        let a356 = v0;
        let a357 = 128u32;
        fixed_u32::bit_and(a356, a357)
    };
    let a359 = 0u32;
    if equal(a358, a359) {
        let a362 = {
            let a360 = v0;
            let a361 = 64u32;
            fixed_u32::bit_and(a360, a361)
        };
        let a363 = 0u32;
        if equal(a362, a363) {
            let a366 = {
                let a364 = v0;
                let a365 = 32u32;
                fixed_u32::bit_and(a364, a365)
            };
            let a367 = 0u32;
            if equal(a366, a367) {
                let a370 = {
                    let a368 = v0;
                    let a369 = 16u32;
                    fixed_u32::bit_and(a368, a369)
                };
                let a371 = 0u32;
                if equal(a370, a371) {
                    let a374 = {
                        let a372 = v0;
                        let a373 = 8u32;
                        fixed_u32::bit_and(a372, a373)
                    };
                    let a375 = 0u32;
                    if equal(a374, a375) {
                        let a378 = {
                            let a376 = v0;
                            let a377 = 4u32;
                            fixed_u32::bit_and(a376, a377)
                        };
                        let a379 = 0u32;
                        if equal(a378, a379) {
                            let a382 = {
                                let a380 = v0;
                                let a381 = 2u32;
                                fixed_u32::bit_and(a380, a381)
                            };
                            let a383 = 0u32;
                            if equal(a382, a383) {
                                let a386 = {
                                    let a384 = v0;
                                    let a385 = 1u32;
                                    fixed_u32::bit_and(a384, a385)
                                };
                                let a387 = 0u32;
                                if equal(a386, a387) {
                                    Bytes::lit(&[0x00])
                                } else {
                                    Bytes::lit(&[0x01])
                                }
                            } else {
                                let a390 = {
                                    let a388 = v0;
                                    let a389 = 1u32;
                                    fixed_u32::bit_and(a388, a389)
                                };
                                let a391 = 0u32;
                                if equal(a390, a391) {
                                    Bytes::lit(&[0x02])
                                } else {
                                    Bytes::lit(&[0x03])
                                }
                            }
                        } else {
                            let a394 = {
                                let a392 = v0;
                                let a393 = 2u32;
                                fixed_u32::bit_and(a392, a393)
                            };
                            let a395 = 0u32;
                            if equal(a394, a395) {
                                let a398 = {
                                    let a396 = v0;
                                    let a397 = 1u32;
                                    fixed_u32::bit_and(a396, a397)
                                };
                                let a399 = 0u32;
                                if equal(a398, a399) {
                                    Bytes::lit(&[0x04])
                                } else {
                                    Bytes::lit(&[0x05])
                                }
                            } else {
                                let a402 = {
                                    let a400 = v0;
                                    let a401 = 1u32;
                                    fixed_u32::bit_and(a400, a401)
                                };
                                let a403 = 0u32;
                                if equal(a402, a403) {
                                    Bytes::lit(&[0x06])
                                } else {
                                    Bytes::lit(&[0x07])
                                }
                            }
                        }
                    } else {
                        let a406 = {
                            let a404 = v0;
                            let a405 = 4u32;
                            fixed_u32::bit_and(a404, a405)
                        };
                        let a407 = 0u32;
                        if equal(a406, a407) {
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
                                    Bytes::lit(&[0x08])
                                } else {
                                    Bytes::lit(&[0x09])
                                }
                            } else {
                                let a418 = {
                                    let a416 = v0;
                                    let a417 = 1u32;
                                    fixed_u32::bit_and(a416, a417)
                                };
                                let a419 = 0u32;
                                if equal(a418, a419) {
                                    Bytes::lit(&[0x0a])
                                } else {
                                    Bytes::lit(&[0x0b])
                                }
                            }
                        } else {
                            let a422 = {
                                let a420 = v0;
                                let a421 = 2u32;
                                fixed_u32::bit_and(a420, a421)
                            };
                            let a423 = 0u32;
                            if equal(a422, a423) {
                                let a426 = {
                                    let a424 = v0;
                                    let a425 = 1u32;
                                    fixed_u32::bit_and(a424, a425)
                                };
                                let a427 = 0u32;
                                if equal(a426, a427) {
                                    Bytes::lit(&[0x0c])
                                } else {
                                    Bytes::lit(&[0x0d])
                                }
                            } else {
                                let a430 = {
                                    let a428 = v0;
                                    let a429 = 1u32;
                                    fixed_u32::bit_and(a428, a429)
                                };
                                let a431 = 0u32;
                                if equal(a430, a431) {
                                    Bytes::lit(&[0x0e])
                                } else {
                                    Bytes::lit(&[0x0f])
                                }
                            }
                        }
                    }
                } else {
                    let a434 = {
                        let a432 = v0;
                        let a433 = 8u32;
                        fixed_u32::bit_and(a432, a433)
                    };
                    let a435 = 0u32;
                    if equal(a434, a435) {
                        let a438 = {
                            let a436 = v0;
                            let a437 = 4u32;
                            fixed_u32::bit_and(a436, a437)
                        };
                        let a439 = 0u32;
                        if equal(a438, a439) {
                            let a442 = {
                                let a440 = v0;
                                let a441 = 2u32;
                                fixed_u32::bit_and(a440, a441)
                            };
                            let a443 = 0u32;
                            if equal(a442, a443) {
                                let a446 = {
                                    let a444 = v0;
                                    let a445 = 1u32;
                                    fixed_u32::bit_and(a444, a445)
                                };
                                let a447 = 0u32;
                                if equal(a446, a447) {
                                    Bytes::lit(&[0x10])
                                } else {
                                    Bytes::lit(&[0x11])
                                }
                            } else {
                                let a450 = {
                                    let a448 = v0;
                                    let a449 = 1u32;
                                    fixed_u32::bit_and(a448, a449)
                                };
                                let a451 = 0u32;
                                if equal(a450, a451) {
                                    Bytes::lit(&[0x12])
                                } else {
                                    Bytes::lit(&[0x13])
                                }
                            }
                        } else {
                            let a454 = {
                                let a452 = v0;
                                let a453 = 2u32;
                                fixed_u32::bit_and(a452, a453)
                            };
                            let a455 = 0u32;
                            if equal(a454, a455) {
                                let a458 = {
                                    let a456 = v0;
                                    let a457 = 1u32;
                                    fixed_u32::bit_and(a456, a457)
                                };
                                let a459 = 0u32;
                                if equal(a458, a459) {
                                    Bytes::lit(&[0x14])
                                } else {
                                    Bytes::lit(&[0x15])
                                }
                            } else {
                                let a462 = {
                                    let a460 = v0;
                                    let a461 = 1u32;
                                    fixed_u32::bit_and(a460, a461)
                                };
                                let a463 = 0u32;
                                if equal(a462, a463) {
                                    Bytes::lit(&[0x16])
                                } else {
                                    Bytes::lit(&[0x17])
                                }
                            }
                        }
                    } else {
                        let a466 = {
                            let a464 = v0;
                            let a465 = 4u32;
                            fixed_u32::bit_and(a464, a465)
                        };
                        let a467 = 0u32;
                        if equal(a466, a467) {
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
                                    Bytes::lit(&[0x18])
                                } else {
                                    Bytes::lit(&[0x19])
                                }
                            } else {
                                let a478 = {
                                    let a476 = v0;
                                    let a477 = 1u32;
                                    fixed_u32::bit_and(a476, a477)
                                };
                                let a479 = 0u32;
                                if equal(a478, a479) {
                                    Bytes::lit(&[0x1a])
                                } else {
                                    Bytes::lit(&[0x1b])
                                }
                            }
                        } else {
                            let a482 = {
                                let a480 = v0;
                                let a481 = 2u32;
                                fixed_u32::bit_and(a480, a481)
                            };
                            let a483 = 0u32;
                            if equal(a482, a483) {
                                let a486 = {
                                    let a484 = v0;
                                    let a485 = 1u32;
                                    fixed_u32::bit_and(a484, a485)
                                };
                                let a487 = 0u32;
                                if equal(a486, a487) {
                                    Bytes::lit(&[0x1c])
                                } else {
                                    Bytes::lit(&[0x1d])
                                }
                            } else {
                                let a490 = {
                                    let a488 = v0;
                                    let a489 = 1u32;
                                    fixed_u32::bit_and(a488, a489)
                                };
                                let a491 = 0u32;
                                if equal(a490, a491) {
                                    Bytes::lit(&[0x1e])
                                } else {
                                    Bytes::lit(&[0x1f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a494 = {
                    let a492 = v0;
                    let a493 = 16u32;
                    fixed_u32::bit_and(a492, a493)
                };
                let a495 = 0u32;
                if equal(a494, a495) {
                    let a498 = {
                        let a496 = v0;
                        let a497 = 8u32;
                        fixed_u32::bit_and(a496, a497)
                    };
                    let a499 = 0u32;
                    if equal(a498, a499) {
                        let a502 = {
                            let a500 = v0;
                            let a501 = 4u32;
                            fixed_u32::bit_and(a500, a501)
                        };
                        let a503 = 0u32;
                        if equal(a502, a503) {
                            let a506 = {
                                let a504 = v0;
                                let a505 = 2u32;
                                fixed_u32::bit_and(a504, a505)
                            };
                            let a507 = 0u32;
                            if equal(a506, a507) {
                                let a510 = {
                                    let a508 = v0;
                                    let a509 = 1u32;
                                    fixed_u32::bit_and(a508, a509)
                                };
                                let a511 = 0u32;
                                if equal(a510, a511) {
                                    Bytes::lit(&[0x20])
                                } else {
                                    Bytes::lit(&[0x21])
                                }
                            } else {
                                let a514 = {
                                    let a512 = v0;
                                    let a513 = 1u32;
                                    fixed_u32::bit_and(a512, a513)
                                };
                                let a515 = 0u32;
                                if equal(a514, a515) {
                                    Bytes::lit(&[0x22])
                                } else {
                                    Bytes::lit(&[0x23])
                                }
                            }
                        } else {
                            let a518 = {
                                let a516 = v0;
                                let a517 = 2u32;
                                fixed_u32::bit_and(a516, a517)
                            };
                            let a519 = 0u32;
                            if equal(a518, a519) {
                                let a522 = {
                                    let a520 = v0;
                                    let a521 = 1u32;
                                    fixed_u32::bit_and(a520, a521)
                                };
                                let a523 = 0u32;
                                if equal(a522, a523) {
                                    Bytes::lit(&[0x24])
                                } else {
                                    Bytes::lit(&[0x25])
                                }
                            } else {
                                let a526 = {
                                    let a524 = v0;
                                    let a525 = 1u32;
                                    fixed_u32::bit_and(a524, a525)
                                };
                                let a527 = 0u32;
                                if equal(a526, a527) {
                                    Bytes::lit(&[0x26])
                                } else {
                                    Bytes::lit(&[0x27])
                                }
                            }
                        }
                    } else {
                        let a530 = {
                            let a528 = v0;
                            let a529 = 4u32;
                            fixed_u32::bit_and(a528, a529)
                        };
                        let a531 = 0u32;
                        if equal(a530, a531) {
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
                                    Bytes::lit(&[0x28])
                                } else {
                                    Bytes::lit(&[0x29])
                                }
                            } else {
                                let a542 = {
                                    let a540 = v0;
                                    let a541 = 1u32;
                                    fixed_u32::bit_and(a540, a541)
                                };
                                let a543 = 0u32;
                                if equal(a542, a543) {
                                    Bytes::lit(&[0x2a])
                                } else {
                                    Bytes::lit(&[0x2b])
                                }
                            }
                        } else {
                            let a546 = {
                                let a544 = v0;
                                let a545 = 2u32;
                                fixed_u32::bit_and(a544, a545)
                            };
                            let a547 = 0u32;
                            if equal(a546, a547) {
                                let a550 = {
                                    let a548 = v0;
                                    let a549 = 1u32;
                                    fixed_u32::bit_and(a548, a549)
                                };
                                let a551 = 0u32;
                                if equal(a550, a551) {
                                    Bytes::lit(&[0x2c])
                                } else {
                                    Bytes::lit(&[0x2d])
                                }
                            } else {
                                let a554 = {
                                    let a552 = v0;
                                    let a553 = 1u32;
                                    fixed_u32::bit_and(a552, a553)
                                };
                                let a555 = 0u32;
                                if equal(a554, a555) {
                                    Bytes::lit(&[0x2e])
                                } else {
                                    Bytes::lit(&[0x2f])
                                }
                            }
                        }
                    }
                } else {
                    let a558 = {
                        let a556 = v0;
                        let a557 = 8u32;
                        fixed_u32::bit_and(a556, a557)
                    };
                    let a559 = 0u32;
                    if equal(a558, a559) {
                        let a562 = {
                            let a560 = v0;
                            let a561 = 4u32;
                            fixed_u32::bit_and(a560, a561)
                        };
                        let a563 = 0u32;
                        if equal(a562, a563) {
                            let a566 = {
                                let a564 = v0;
                                let a565 = 2u32;
                                fixed_u32::bit_and(a564, a565)
                            };
                            let a567 = 0u32;
                            if equal(a566, a567) {
                                let a570 = {
                                    let a568 = v0;
                                    let a569 = 1u32;
                                    fixed_u32::bit_and(a568, a569)
                                };
                                let a571 = 0u32;
                                if equal(a570, a571) {
                                    Bytes::lit(&[0x30])
                                } else {
                                    Bytes::lit(&[0x31])
                                }
                            } else {
                                let a574 = {
                                    let a572 = v0;
                                    let a573 = 1u32;
                                    fixed_u32::bit_and(a572, a573)
                                };
                                let a575 = 0u32;
                                if equal(a574, a575) {
                                    Bytes::lit(&[0x32])
                                } else {
                                    Bytes::lit(&[0x33])
                                }
                            }
                        } else {
                            let a578 = {
                                let a576 = v0;
                                let a577 = 2u32;
                                fixed_u32::bit_and(a576, a577)
                            };
                            let a579 = 0u32;
                            if equal(a578, a579) {
                                let a582 = {
                                    let a580 = v0;
                                    let a581 = 1u32;
                                    fixed_u32::bit_and(a580, a581)
                                };
                                let a583 = 0u32;
                                if equal(a582, a583) {
                                    Bytes::lit(&[0x34])
                                } else {
                                    Bytes::lit(&[0x35])
                                }
                            } else {
                                let a586 = {
                                    let a584 = v0;
                                    let a585 = 1u32;
                                    fixed_u32::bit_and(a584, a585)
                                };
                                let a587 = 0u32;
                                if equal(a586, a587) {
                                    Bytes::lit(&[0x36])
                                } else {
                                    Bytes::lit(&[0x37])
                                }
                            }
                        }
                    } else {
                        let a590 = {
                            let a588 = v0;
                            let a589 = 4u32;
                            fixed_u32::bit_and(a588, a589)
                        };
                        let a591 = 0u32;
                        if equal(a590, a591) {
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
                                    Bytes::lit(&[0x38])
                                } else {
                                    Bytes::lit(&[0x39])
                                }
                            } else {
                                let a602 = {
                                    let a600 = v0;
                                    let a601 = 1u32;
                                    fixed_u32::bit_and(a600, a601)
                                };
                                let a603 = 0u32;
                                if equal(a602, a603) {
                                    Bytes::lit(&[0x3a])
                                } else {
                                    Bytes::lit(&[0x3b])
                                }
                            }
                        } else {
                            let a606 = {
                                let a604 = v0;
                                let a605 = 2u32;
                                fixed_u32::bit_and(a604, a605)
                            };
                            let a607 = 0u32;
                            if equal(a606, a607) {
                                let a610 = {
                                    let a608 = v0;
                                    let a609 = 1u32;
                                    fixed_u32::bit_and(a608, a609)
                                };
                                let a611 = 0u32;
                                if equal(a610, a611) {
                                    Bytes::lit(&[0x3c])
                                } else {
                                    Bytes::lit(&[0x3d])
                                }
                            } else {
                                let a614 = {
                                    let a612 = v0;
                                    let a613 = 1u32;
                                    fixed_u32::bit_and(a612, a613)
                                };
                                let a615 = 0u32;
                                if equal(a614, a615) {
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
            let a618 = {
                let a616 = v0;
                let a617 = 32u32;
                fixed_u32::bit_and(a616, a617)
            };
            let a619 = 0u32;
            if equal(a618, a619) {
                let a622 = {
                    let a620 = v0;
                    let a621 = 16u32;
                    fixed_u32::bit_and(a620, a621)
                };
                let a623 = 0u32;
                if equal(a622, a623) {
                    let a626 = {
                        let a624 = v0;
                        let a625 = 8u32;
                        fixed_u32::bit_and(a624, a625)
                    };
                    let a627 = 0u32;
                    if equal(a626, a627) {
                        let a630 = {
                            let a628 = v0;
                            let a629 = 4u32;
                            fixed_u32::bit_and(a628, a629)
                        };
                        let a631 = 0u32;
                        if equal(a630, a631) {
                            let a634 = {
                                let a632 = v0;
                                let a633 = 2u32;
                                fixed_u32::bit_and(a632, a633)
                            };
                            let a635 = 0u32;
                            if equal(a634, a635) {
                                let a638 = {
                                    let a636 = v0;
                                    let a637 = 1u32;
                                    fixed_u32::bit_and(a636, a637)
                                };
                                let a639 = 0u32;
                                if equal(a638, a639) {
                                    Bytes::lit(&[0x40])
                                } else {
                                    Bytes::lit(&[0x41])
                                }
                            } else {
                                let a642 = {
                                    let a640 = v0;
                                    let a641 = 1u32;
                                    fixed_u32::bit_and(a640, a641)
                                };
                                let a643 = 0u32;
                                if equal(a642, a643) {
                                    Bytes::lit(&[0x42])
                                } else {
                                    Bytes::lit(&[0x43])
                                }
                            }
                        } else {
                            let a646 = {
                                let a644 = v0;
                                let a645 = 2u32;
                                fixed_u32::bit_and(a644, a645)
                            };
                            let a647 = 0u32;
                            if equal(a646, a647) {
                                let a650 = {
                                    let a648 = v0;
                                    let a649 = 1u32;
                                    fixed_u32::bit_and(a648, a649)
                                };
                                let a651 = 0u32;
                                if equal(a650, a651) {
                                    Bytes::lit(&[0x44])
                                } else {
                                    Bytes::lit(&[0x45])
                                }
                            } else {
                                let a654 = {
                                    let a652 = v0;
                                    let a653 = 1u32;
                                    fixed_u32::bit_and(a652, a653)
                                };
                                let a655 = 0u32;
                                if equal(a654, a655) {
                                    Bytes::lit(&[0x46])
                                } else {
                                    Bytes::lit(&[0x47])
                                }
                            }
                        }
                    } else {
                        let a658 = {
                            let a656 = v0;
                            let a657 = 4u32;
                            fixed_u32::bit_and(a656, a657)
                        };
                        let a659 = 0u32;
                        if equal(a658, a659) {
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
                                    Bytes::lit(&[0x48])
                                } else {
                                    Bytes::lit(&[0x49])
                                }
                            } else {
                                let a670 = {
                                    let a668 = v0;
                                    let a669 = 1u32;
                                    fixed_u32::bit_and(a668, a669)
                                };
                                let a671 = 0u32;
                                if equal(a670, a671) {
                                    Bytes::lit(&[0x4a])
                                } else {
                                    Bytes::lit(&[0x4b])
                                }
                            }
                        } else {
                            let a674 = {
                                let a672 = v0;
                                let a673 = 2u32;
                                fixed_u32::bit_and(a672, a673)
                            };
                            let a675 = 0u32;
                            if equal(a674, a675) {
                                let a678 = {
                                    let a676 = v0;
                                    let a677 = 1u32;
                                    fixed_u32::bit_and(a676, a677)
                                };
                                let a679 = 0u32;
                                if equal(a678, a679) {
                                    Bytes::lit(&[0x4c])
                                } else {
                                    Bytes::lit(&[0x4d])
                                }
                            } else {
                                let a682 = {
                                    let a680 = v0;
                                    let a681 = 1u32;
                                    fixed_u32::bit_and(a680, a681)
                                };
                                let a683 = 0u32;
                                if equal(a682, a683) {
                                    Bytes::lit(&[0x4e])
                                } else {
                                    Bytes::lit(&[0x4f])
                                }
                            }
                        }
                    }
                } else {
                    let a686 = {
                        let a684 = v0;
                        let a685 = 8u32;
                        fixed_u32::bit_and(a684, a685)
                    };
                    let a687 = 0u32;
                    if equal(a686, a687) {
                        let a690 = {
                            let a688 = v0;
                            let a689 = 4u32;
                            fixed_u32::bit_and(a688, a689)
                        };
                        let a691 = 0u32;
                        if equal(a690, a691) {
                            let a694 = {
                                let a692 = v0;
                                let a693 = 2u32;
                                fixed_u32::bit_and(a692, a693)
                            };
                            let a695 = 0u32;
                            if equal(a694, a695) {
                                let a698 = {
                                    let a696 = v0;
                                    let a697 = 1u32;
                                    fixed_u32::bit_and(a696, a697)
                                };
                                let a699 = 0u32;
                                if equal(a698, a699) {
                                    Bytes::lit(&[0x50])
                                } else {
                                    Bytes::lit(&[0x51])
                                }
                            } else {
                                let a702 = {
                                    let a700 = v0;
                                    let a701 = 1u32;
                                    fixed_u32::bit_and(a700, a701)
                                };
                                let a703 = 0u32;
                                if equal(a702, a703) {
                                    Bytes::lit(&[0x52])
                                } else {
                                    Bytes::lit(&[0x53])
                                }
                            }
                        } else {
                            let a706 = {
                                let a704 = v0;
                                let a705 = 2u32;
                                fixed_u32::bit_and(a704, a705)
                            };
                            let a707 = 0u32;
                            if equal(a706, a707) {
                                let a710 = {
                                    let a708 = v0;
                                    let a709 = 1u32;
                                    fixed_u32::bit_and(a708, a709)
                                };
                                let a711 = 0u32;
                                if equal(a710, a711) {
                                    Bytes::lit(&[0x54])
                                } else {
                                    Bytes::lit(&[0x55])
                                }
                            } else {
                                let a714 = {
                                    let a712 = v0;
                                    let a713 = 1u32;
                                    fixed_u32::bit_and(a712, a713)
                                };
                                let a715 = 0u32;
                                if equal(a714, a715) {
                                    Bytes::lit(&[0x56])
                                } else {
                                    Bytes::lit(&[0x57])
                                }
                            }
                        }
                    } else {
                        let a718 = {
                            let a716 = v0;
                            let a717 = 4u32;
                            fixed_u32::bit_and(a716, a717)
                        };
                        let a719 = 0u32;
                        if equal(a718, a719) {
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
                                    Bytes::lit(&[0x58])
                                } else {
                                    Bytes::lit(&[0x59])
                                }
                            } else {
                                let a730 = {
                                    let a728 = v0;
                                    let a729 = 1u32;
                                    fixed_u32::bit_and(a728, a729)
                                };
                                let a731 = 0u32;
                                if equal(a730, a731) {
                                    Bytes::lit(&[0x5a])
                                } else {
                                    Bytes::lit(&[0x5b])
                                }
                            }
                        } else {
                            let a734 = {
                                let a732 = v0;
                                let a733 = 2u32;
                                fixed_u32::bit_and(a732, a733)
                            };
                            let a735 = 0u32;
                            if equal(a734, a735) {
                                let a738 = {
                                    let a736 = v0;
                                    let a737 = 1u32;
                                    fixed_u32::bit_and(a736, a737)
                                };
                                let a739 = 0u32;
                                if equal(a738, a739) {
                                    Bytes::lit(&[0x5c])
                                } else {
                                    Bytes::lit(&[0x5d])
                                }
                            } else {
                                let a742 = {
                                    let a740 = v0;
                                    let a741 = 1u32;
                                    fixed_u32::bit_and(a740, a741)
                                };
                                let a743 = 0u32;
                                if equal(a742, a743) {
                                    Bytes::lit(&[0x5e])
                                } else {
                                    Bytes::lit(&[0x5f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a746 = {
                    let a744 = v0;
                    let a745 = 16u32;
                    fixed_u32::bit_and(a744, a745)
                };
                let a747 = 0u32;
                if equal(a746, a747) {
                    let a750 = {
                        let a748 = v0;
                        let a749 = 8u32;
                        fixed_u32::bit_and(a748, a749)
                    };
                    let a751 = 0u32;
                    if equal(a750, a751) {
                        let a754 = {
                            let a752 = v0;
                            let a753 = 4u32;
                            fixed_u32::bit_and(a752, a753)
                        };
                        let a755 = 0u32;
                        if equal(a754, a755) {
                            let a758 = {
                                let a756 = v0;
                                let a757 = 2u32;
                                fixed_u32::bit_and(a756, a757)
                            };
                            let a759 = 0u32;
                            if equal(a758, a759) {
                                let a762 = {
                                    let a760 = v0;
                                    let a761 = 1u32;
                                    fixed_u32::bit_and(a760, a761)
                                };
                                let a763 = 0u32;
                                if equal(a762, a763) {
                                    Bytes::lit(&[0x60])
                                } else {
                                    Bytes::lit(&[0x61])
                                }
                            } else {
                                let a766 = {
                                    let a764 = v0;
                                    let a765 = 1u32;
                                    fixed_u32::bit_and(a764, a765)
                                };
                                let a767 = 0u32;
                                if equal(a766, a767) {
                                    Bytes::lit(&[0x62])
                                } else {
                                    Bytes::lit(&[0x63])
                                }
                            }
                        } else {
                            let a770 = {
                                let a768 = v0;
                                let a769 = 2u32;
                                fixed_u32::bit_and(a768, a769)
                            };
                            let a771 = 0u32;
                            if equal(a770, a771) {
                                let a774 = {
                                    let a772 = v0;
                                    let a773 = 1u32;
                                    fixed_u32::bit_and(a772, a773)
                                };
                                let a775 = 0u32;
                                if equal(a774, a775) {
                                    Bytes::lit(&[0x64])
                                } else {
                                    Bytes::lit(&[0x65])
                                }
                            } else {
                                let a778 = {
                                    let a776 = v0;
                                    let a777 = 1u32;
                                    fixed_u32::bit_and(a776, a777)
                                };
                                let a779 = 0u32;
                                if equal(a778, a779) {
                                    Bytes::lit(&[0x66])
                                } else {
                                    Bytes::lit(&[0x67])
                                }
                            }
                        }
                    } else {
                        let a782 = {
                            let a780 = v0;
                            let a781 = 4u32;
                            fixed_u32::bit_and(a780, a781)
                        };
                        let a783 = 0u32;
                        if equal(a782, a783) {
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
                                    Bytes::lit(&[0x68])
                                } else {
                                    Bytes::lit(&[0x69])
                                }
                            } else {
                                let a794 = {
                                    let a792 = v0;
                                    let a793 = 1u32;
                                    fixed_u32::bit_and(a792, a793)
                                };
                                let a795 = 0u32;
                                if equal(a794, a795) {
                                    Bytes::lit(&[0x6a])
                                } else {
                                    Bytes::lit(&[0x6b])
                                }
                            }
                        } else {
                            let a798 = {
                                let a796 = v0;
                                let a797 = 2u32;
                                fixed_u32::bit_and(a796, a797)
                            };
                            let a799 = 0u32;
                            if equal(a798, a799) {
                                let a802 = {
                                    let a800 = v0;
                                    let a801 = 1u32;
                                    fixed_u32::bit_and(a800, a801)
                                };
                                let a803 = 0u32;
                                if equal(a802, a803) {
                                    Bytes::lit(&[0x6c])
                                } else {
                                    Bytes::lit(&[0x6d])
                                }
                            } else {
                                let a806 = {
                                    let a804 = v0;
                                    let a805 = 1u32;
                                    fixed_u32::bit_and(a804, a805)
                                };
                                let a807 = 0u32;
                                if equal(a806, a807) {
                                    Bytes::lit(&[0x6e])
                                } else {
                                    Bytes::lit(&[0x6f])
                                }
                            }
                        }
                    }
                } else {
                    let a810 = {
                        let a808 = v0;
                        let a809 = 8u32;
                        fixed_u32::bit_and(a808, a809)
                    };
                    let a811 = 0u32;
                    if equal(a810, a811) {
                        let a814 = {
                            let a812 = v0;
                            let a813 = 4u32;
                            fixed_u32::bit_and(a812, a813)
                        };
                        let a815 = 0u32;
                        if equal(a814, a815) {
                            let a818 = {
                                let a816 = v0;
                                let a817 = 2u32;
                                fixed_u32::bit_and(a816, a817)
                            };
                            let a819 = 0u32;
                            if equal(a818, a819) {
                                let a822 = {
                                    let a820 = v0;
                                    let a821 = 1u32;
                                    fixed_u32::bit_and(a820, a821)
                                };
                                let a823 = 0u32;
                                if equal(a822, a823) {
                                    Bytes::lit(&[0x70])
                                } else {
                                    Bytes::lit(&[0x71])
                                }
                            } else {
                                let a826 = {
                                    let a824 = v0;
                                    let a825 = 1u32;
                                    fixed_u32::bit_and(a824, a825)
                                };
                                let a827 = 0u32;
                                if equal(a826, a827) {
                                    Bytes::lit(&[0x72])
                                } else {
                                    Bytes::lit(&[0x73])
                                }
                            }
                        } else {
                            let a830 = {
                                let a828 = v0;
                                let a829 = 2u32;
                                fixed_u32::bit_and(a828, a829)
                            };
                            let a831 = 0u32;
                            if equal(a830, a831) {
                                let a834 = {
                                    let a832 = v0;
                                    let a833 = 1u32;
                                    fixed_u32::bit_and(a832, a833)
                                };
                                let a835 = 0u32;
                                if equal(a834, a835) {
                                    Bytes::lit(&[0x74])
                                } else {
                                    Bytes::lit(&[0x75])
                                }
                            } else {
                                let a838 = {
                                    let a836 = v0;
                                    let a837 = 1u32;
                                    fixed_u32::bit_and(a836, a837)
                                };
                                let a839 = 0u32;
                                if equal(a838, a839) {
                                    Bytes::lit(&[0x76])
                                } else {
                                    Bytes::lit(&[0x77])
                                }
                            }
                        }
                    } else {
                        let a842 = {
                            let a840 = v0;
                            let a841 = 4u32;
                            fixed_u32::bit_and(a840, a841)
                        };
                        let a843 = 0u32;
                        if equal(a842, a843) {
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
                                    Bytes::lit(&[0x78])
                                } else {
                                    Bytes::lit(&[0x79])
                                }
                            } else {
                                let a854 = {
                                    let a852 = v0;
                                    let a853 = 1u32;
                                    fixed_u32::bit_and(a852, a853)
                                };
                                let a855 = 0u32;
                                if equal(a854, a855) {
                                    Bytes::lit(&[0x7a])
                                } else {
                                    Bytes::lit(&[0x7b])
                                }
                            }
                        } else {
                            let a858 = {
                                let a856 = v0;
                                let a857 = 2u32;
                                fixed_u32::bit_and(a856, a857)
                            };
                            let a859 = 0u32;
                            if equal(a858, a859) {
                                let a862 = {
                                    let a860 = v0;
                                    let a861 = 1u32;
                                    fixed_u32::bit_and(a860, a861)
                                };
                                let a863 = 0u32;
                                if equal(a862, a863) {
                                    Bytes::lit(&[0x7c])
                                } else {
                                    Bytes::lit(&[0x7d])
                                }
                            } else {
                                let a866 = {
                                    let a864 = v0;
                                    let a865 = 1u32;
                                    fixed_u32::bit_and(a864, a865)
                                };
                                let a867 = 0u32;
                                if equal(a866, a867) {
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
        let a870 = {
            let a868 = v0;
            let a869 = 64u32;
            fixed_u32::bit_and(a868, a869)
        };
        let a871 = 0u32;
        if equal(a870, a871) {
            let a874 = {
                let a872 = v0;
                let a873 = 32u32;
                fixed_u32::bit_and(a872, a873)
            };
            let a875 = 0u32;
            if equal(a874, a875) {
                let a878 = {
                    let a876 = v0;
                    let a877 = 16u32;
                    fixed_u32::bit_and(a876, a877)
                };
                let a879 = 0u32;
                if equal(a878, a879) {
                    let a882 = {
                        let a880 = v0;
                        let a881 = 8u32;
                        fixed_u32::bit_and(a880, a881)
                    };
                    let a883 = 0u32;
                    if equal(a882, a883) {
                        let a886 = {
                            let a884 = v0;
                            let a885 = 4u32;
                            fixed_u32::bit_and(a884, a885)
                        };
                        let a887 = 0u32;
                        if equal(a886, a887) {
                            let a890 = {
                                let a888 = v0;
                                let a889 = 2u32;
                                fixed_u32::bit_and(a888, a889)
                            };
                            let a891 = 0u32;
                            if equal(a890, a891) {
                                let a894 = {
                                    let a892 = v0;
                                    let a893 = 1u32;
                                    fixed_u32::bit_and(a892, a893)
                                };
                                let a895 = 0u32;
                                if equal(a894, a895) {
                                    Bytes::lit(&[0x80])
                                } else {
                                    Bytes::lit(&[0x81])
                                }
                            } else {
                                let a898 = {
                                    let a896 = v0;
                                    let a897 = 1u32;
                                    fixed_u32::bit_and(a896, a897)
                                };
                                let a899 = 0u32;
                                if equal(a898, a899) {
                                    Bytes::lit(&[0x82])
                                } else {
                                    Bytes::lit(&[0x83])
                                }
                            }
                        } else {
                            let a902 = {
                                let a900 = v0;
                                let a901 = 2u32;
                                fixed_u32::bit_and(a900, a901)
                            };
                            let a903 = 0u32;
                            if equal(a902, a903) {
                                let a906 = {
                                    let a904 = v0;
                                    let a905 = 1u32;
                                    fixed_u32::bit_and(a904, a905)
                                };
                                let a907 = 0u32;
                                if equal(a906, a907) {
                                    Bytes::lit(&[0x84])
                                } else {
                                    Bytes::lit(&[0x85])
                                }
                            } else {
                                let a910 = {
                                    let a908 = v0;
                                    let a909 = 1u32;
                                    fixed_u32::bit_and(a908, a909)
                                };
                                let a911 = 0u32;
                                if equal(a910, a911) {
                                    Bytes::lit(&[0x86])
                                } else {
                                    Bytes::lit(&[0x87])
                                }
                            }
                        }
                    } else {
                        let a914 = {
                            let a912 = v0;
                            let a913 = 4u32;
                            fixed_u32::bit_and(a912, a913)
                        };
                        let a915 = 0u32;
                        if equal(a914, a915) {
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
                                    Bytes::lit(&[0x88])
                                } else {
                                    Bytes::lit(&[0x89])
                                }
                            } else {
                                let a926 = {
                                    let a924 = v0;
                                    let a925 = 1u32;
                                    fixed_u32::bit_and(a924, a925)
                                };
                                let a927 = 0u32;
                                if equal(a926, a927) {
                                    Bytes::lit(&[0x8a])
                                } else {
                                    Bytes::lit(&[0x8b])
                                }
                            }
                        } else {
                            let a930 = {
                                let a928 = v0;
                                let a929 = 2u32;
                                fixed_u32::bit_and(a928, a929)
                            };
                            let a931 = 0u32;
                            if equal(a930, a931) {
                                let a934 = {
                                    let a932 = v0;
                                    let a933 = 1u32;
                                    fixed_u32::bit_and(a932, a933)
                                };
                                let a935 = 0u32;
                                if equal(a934, a935) {
                                    Bytes::lit(&[0x8c])
                                } else {
                                    Bytes::lit(&[0x8d])
                                }
                            } else {
                                let a938 = {
                                    let a936 = v0;
                                    let a937 = 1u32;
                                    fixed_u32::bit_and(a936, a937)
                                };
                                let a939 = 0u32;
                                if equal(a938, a939) {
                                    Bytes::lit(&[0x8e])
                                } else {
                                    Bytes::lit(&[0x8f])
                                }
                            }
                        }
                    }
                } else {
                    let a942 = {
                        let a940 = v0;
                        let a941 = 8u32;
                        fixed_u32::bit_and(a940, a941)
                    };
                    let a943 = 0u32;
                    if equal(a942, a943) {
                        let a946 = {
                            let a944 = v0;
                            let a945 = 4u32;
                            fixed_u32::bit_and(a944, a945)
                        };
                        let a947 = 0u32;
                        if equal(a946, a947) {
                            let a950 = {
                                let a948 = v0;
                                let a949 = 2u32;
                                fixed_u32::bit_and(a948, a949)
                            };
                            let a951 = 0u32;
                            if equal(a950, a951) {
                                let a954 = {
                                    let a952 = v0;
                                    let a953 = 1u32;
                                    fixed_u32::bit_and(a952, a953)
                                };
                                let a955 = 0u32;
                                if equal(a954, a955) {
                                    Bytes::lit(&[0x90])
                                } else {
                                    Bytes::lit(&[0x91])
                                }
                            } else {
                                let a958 = {
                                    let a956 = v0;
                                    let a957 = 1u32;
                                    fixed_u32::bit_and(a956, a957)
                                };
                                let a959 = 0u32;
                                if equal(a958, a959) {
                                    Bytes::lit(&[0x92])
                                } else {
                                    Bytes::lit(&[0x93])
                                }
                            }
                        } else {
                            let a962 = {
                                let a960 = v0;
                                let a961 = 2u32;
                                fixed_u32::bit_and(a960, a961)
                            };
                            let a963 = 0u32;
                            if equal(a962, a963) {
                                let a966 = {
                                    let a964 = v0;
                                    let a965 = 1u32;
                                    fixed_u32::bit_and(a964, a965)
                                };
                                let a967 = 0u32;
                                if equal(a966, a967) {
                                    Bytes::lit(&[0x94])
                                } else {
                                    Bytes::lit(&[0x95])
                                }
                            } else {
                                let a970 = {
                                    let a968 = v0;
                                    let a969 = 1u32;
                                    fixed_u32::bit_and(a968, a969)
                                };
                                let a971 = 0u32;
                                if equal(a970, a971) {
                                    Bytes::lit(&[0x96])
                                } else {
                                    Bytes::lit(&[0x97])
                                }
                            }
                        }
                    } else {
                        let a974 = {
                            let a972 = v0;
                            let a973 = 4u32;
                            fixed_u32::bit_and(a972, a973)
                        };
                        let a975 = 0u32;
                        if equal(a974, a975) {
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
                                    Bytes::lit(&[0x98])
                                } else {
                                    Bytes::lit(&[0x99])
                                }
                            } else {
                                let a986 = {
                                    let a984 = v0;
                                    let a985 = 1u32;
                                    fixed_u32::bit_and(a984, a985)
                                };
                                let a987 = 0u32;
                                if equal(a986, a987) {
                                    Bytes::lit(&[0x9a])
                                } else {
                                    Bytes::lit(&[0x9b])
                                }
                            }
                        } else {
                            let a990 = {
                                let a988 = v0;
                                let a989 = 2u32;
                                fixed_u32::bit_and(a988, a989)
                            };
                            let a991 = 0u32;
                            if equal(a990, a991) {
                                let a994 = {
                                    let a992 = v0;
                                    let a993 = 1u32;
                                    fixed_u32::bit_and(a992, a993)
                                };
                                let a995 = 0u32;
                                if equal(a994, a995) {
                                    Bytes::lit(&[0x9c])
                                } else {
                                    Bytes::lit(&[0x9d])
                                }
                            } else {
                                let a998 = {
                                    let a996 = v0;
                                    let a997 = 1u32;
                                    fixed_u32::bit_and(a996, a997)
                                };
                                let a999 = 0u32;
                                if equal(a998, a999) {
                                    Bytes::lit(&[0x9e])
                                } else {
                                    Bytes::lit(&[0x9f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a1002 = {
                    let a1000 = v0;
                    let a1001 = 16u32;
                    fixed_u32::bit_and(a1000, a1001)
                };
                let a1003 = 0u32;
                if equal(a1002, a1003) {
                    let a1006 = {
                        let a1004 = v0;
                        let a1005 = 8u32;
                        fixed_u32::bit_and(a1004, a1005)
                    };
                    let a1007 = 0u32;
                    if equal(a1006, a1007) {
                        let a1010 = {
                            let a1008 = v0;
                            let a1009 = 4u32;
                            fixed_u32::bit_and(a1008, a1009)
                        };
                        let a1011 = 0u32;
                        if equal(a1010, a1011) {
                            let a1014 = {
                                let a1012 = v0;
                                let a1013 = 2u32;
                                fixed_u32::bit_and(a1012, a1013)
                            };
                            let a1015 = 0u32;
                            if equal(a1014, a1015) {
                                let a1018 = {
                                    let a1016 = v0;
                                    let a1017 = 1u32;
                                    fixed_u32::bit_and(a1016, a1017)
                                };
                                let a1019 = 0u32;
                                if equal(a1018, a1019) {
                                    Bytes::lit(&[0xa0])
                                } else {
                                    Bytes::lit(&[0xa1])
                                }
                            } else {
                                let a1022 = {
                                    let a1020 = v0;
                                    let a1021 = 1u32;
                                    fixed_u32::bit_and(a1020, a1021)
                                };
                                let a1023 = 0u32;
                                if equal(a1022, a1023) {
                                    Bytes::lit(&[0xa2])
                                } else {
                                    Bytes::lit(&[0xa3])
                                }
                            }
                        } else {
                            let a1026 = {
                                let a1024 = v0;
                                let a1025 = 2u32;
                                fixed_u32::bit_and(a1024, a1025)
                            };
                            let a1027 = 0u32;
                            if equal(a1026, a1027) {
                                let a1030 = {
                                    let a1028 = v0;
                                    let a1029 = 1u32;
                                    fixed_u32::bit_and(a1028, a1029)
                                };
                                let a1031 = 0u32;
                                if equal(a1030, a1031) {
                                    Bytes::lit(&[0xa4])
                                } else {
                                    Bytes::lit(&[0xa5])
                                }
                            } else {
                                let a1034 = {
                                    let a1032 = v0;
                                    let a1033 = 1u32;
                                    fixed_u32::bit_and(a1032, a1033)
                                };
                                let a1035 = 0u32;
                                if equal(a1034, a1035) {
                                    Bytes::lit(&[0xa6])
                                } else {
                                    Bytes::lit(&[0xa7])
                                }
                            }
                        }
                    } else {
                        let a1038 = {
                            let a1036 = v0;
                            let a1037 = 4u32;
                            fixed_u32::bit_and(a1036, a1037)
                        };
                        let a1039 = 0u32;
                        if equal(a1038, a1039) {
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
                                    Bytes::lit(&[0xa8])
                                } else {
                                    Bytes::lit(&[0xa9])
                                }
                            } else {
                                let a1050 = {
                                    let a1048 = v0;
                                    let a1049 = 1u32;
                                    fixed_u32::bit_and(a1048, a1049)
                                };
                                let a1051 = 0u32;
                                if equal(a1050, a1051) {
                                    Bytes::lit(&[0xaa])
                                } else {
                                    Bytes::lit(&[0xab])
                                }
                            }
                        } else {
                            let a1054 = {
                                let a1052 = v0;
                                let a1053 = 2u32;
                                fixed_u32::bit_and(a1052, a1053)
                            };
                            let a1055 = 0u32;
                            if equal(a1054, a1055) {
                                let a1058 = {
                                    let a1056 = v0;
                                    let a1057 = 1u32;
                                    fixed_u32::bit_and(a1056, a1057)
                                };
                                let a1059 = 0u32;
                                if equal(a1058, a1059) {
                                    Bytes::lit(&[0xac])
                                } else {
                                    Bytes::lit(&[0xad])
                                }
                            } else {
                                let a1062 = {
                                    let a1060 = v0;
                                    let a1061 = 1u32;
                                    fixed_u32::bit_and(a1060, a1061)
                                };
                                let a1063 = 0u32;
                                if equal(a1062, a1063) {
                                    Bytes::lit(&[0xae])
                                } else {
                                    Bytes::lit(&[0xaf])
                                }
                            }
                        }
                    }
                } else {
                    let a1066 = {
                        let a1064 = v0;
                        let a1065 = 8u32;
                        fixed_u32::bit_and(a1064, a1065)
                    };
                    let a1067 = 0u32;
                    if equal(a1066, a1067) {
                        let a1070 = {
                            let a1068 = v0;
                            let a1069 = 4u32;
                            fixed_u32::bit_and(a1068, a1069)
                        };
                        let a1071 = 0u32;
                        if equal(a1070, a1071) {
                            let a1074 = {
                                let a1072 = v0;
                                let a1073 = 2u32;
                                fixed_u32::bit_and(a1072, a1073)
                            };
                            let a1075 = 0u32;
                            if equal(a1074, a1075) {
                                let a1078 = {
                                    let a1076 = v0;
                                    let a1077 = 1u32;
                                    fixed_u32::bit_and(a1076, a1077)
                                };
                                let a1079 = 0u32;
                                if equal(a1078, a1079) {
                                    Bytes::lit(&[0xb0])
                                } else {
                                    Bytes::lit(&[0xb1])
                                }
                            } else {
                                let a1082 = {
                                    let a1080 = v0;
                                    let a1081 = 1u32;
                                    fixed_u32::bit_and(a1080, a1081)
                                };
                                let a1083 = 0u32;
                                if equal(a1082, a1083) {
                                    Bytes::lit(&[0xb2])
                                } else {
                                    Bytes::lit(&[0xb3])
                                }
                            }
                        } else {
                            let a1086 = {
                                let a1084 = v0;
                                let a1085 = 2u32;
                                fixed_u32::bit_and(a1084, a1085)
                            };
                            let a1087 = 0u32;
                            if equal(a1086, a1087) {
                                let a1090 = {
                                    let a1088 = v0;
                                    let a1089 = 1u32;
                                    fixed_u32::bit_and(a1088, a1089)
                                };
                                let a1091 = 0u32;
                                if equal(a1090, a1091) {
                                    Bytes::lit(&[0xb4])
                                } else {
                                    Bytes::lit(&[0xb5])
                                }
                            } else {
                                let a1094 = {
                                    let a1092 = v0;
                                    let a1093 = 1u32;
                                    fixed_u32::bit_and(a1092, a1093)
                                };
                                let a1095 = 0u32;
                                if equal(a1094, a1095) {
                                    Bytes::lit(&[0xb6])
                                } else {
                                    Bytes::lit(&[0xb7])
                                }
                            }
                        }
                    } else {
                        let a1098 = {
                            let a1096 = v0;
                            let a1097 = 4u32;
                            fixed_u32::bit_and(a1096, a1097)
                        };
                        let a1099 = 0u32;
                        if equal(a1098, a1099) {
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
                                    Bytes::lit(&[0xb8])
                                } else {
                                    Bytes::lit(&[0xb9])
                                }
                            } else {
                                let a1110 = {
                                    let a1108 = v0;
                                    let a1109 = 1u32;
                                    fixed_u32::bit_and(a1108, a1109)
                                };
                                let a1111 = 0u32;
                                if equal(a1110, a1111) {
                                    Bytes::lit(&[0xba])
                                } else {
                                    Bytes::lit(&[0xbb])
                                }
                            }
                        } else {
                            let a1114 = {
                                let a1112 = v0;
                                let a1113 = 2u32;
                                fixed_u32::bit_and(a1112, a1113)
                            };
                            let a1115 = 0u32;
                            if equal(a1114, a1115) {
                                let a1118 = {
                                    let a1116 = v0;
                                    let a1117 = 1u32;
                                    fixed_u32::bit_and(a1116, a1117)
                                };
                                let a1119 = 0u32;
                                if equal(a1118, a1119) {
                                    Bytes::lit(&[0xbc])
                                } else {
                                    Bytes::lit(&[0xbd])
                                }
                            } else {
                                let a1122 = {
                                    let a1120 = v0;
                                    let a1121 = 1u32;
                                    fixed_u32::bit_and(a1120, a1121)
                                };
                                let a1123 = 0u32;
                                if equal(a1122, a1123) {
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
            let a1126 = {
                let a1124 = v0;
                let a1125 = 32u32;
                fixed_u32::bit_and(a1124, a1125)
            };
            let a1127 = 0u32;
            if equal(a1126, a1127) {
                let a1130 = {
                    let a1128 = v0;
                    let a1129 = 16u32;
                    fixed_u32::bit_and(a1128, a1129)
                };
                let a1131 = 0u32;
                if equal(a1130, a1131) {
                    let a1134 = {
                        let a1132 = v0;
                        let a1133 = 8u32;
                        fixed_u32::bit_and(a1132, a1133)
                    };
                    let a1135 = 0u32;
                    if equal(a1134, a1135) {
                        let a1138 = {
                            let a1136 = v0;
                            let a1137 = 4u32;
                            fixed_u32::bit_and(a1136, a1137)
                        };
                        let a1139 = 0u32;
                        if equal(a1138, a1139) {
                            let a1142 = {
                                let a1140 = v0;
                                let a1141 = 2u32;
                                fixed_u32::bit_and(a1140, a1141)
                            };
                            let a1143 = 0u32;
                            if equal(a1142, a1143) {
                                let a1146 = {
                                    let a1144 = v0;
                                    let a1145 = 1u32;
                                    fixed_u32::bit_and(a1144, a1145)
                                };
                                let a1147 = 0u32;
                                if equal(a1146, a1147) {
                                    Bytes::lit(&[0xc0])
                                } else {
                                    Bytes::lit(&[0xc1])
                                }
                            } else {
                                let a1150 = {
                                    let a1148 = v0;
                                    let a1149 = 1u32;
                                    fixed_u32::bit_and(a1148, a1149)
                                };
                                let a1151 = 0u32;
                                if equal(a1150, a1151) {
                                    Bytes::lit(&[0xc2])
                                } else {
                                    Bytes::lit(&[0xc3])
                                }
                            }
                        } else {
                            let a1154 = {
                                let a1152 = v0;
                                let a1153 = 2u32;
                                fixed_u32::bit_and(a1152, a1153)
                            };
                            let a1155 = 0u32;
                            if equal(a1154, a1155) {
                                let a1158 = {
                                    let a1156 = v0;
                                    let a1157 = 1u32;
                                    fixed_u32::bit_and(a1156, a1157)
                                };
                                let a1159 = 0u32;
                                if equal(a1158, a1159) {
                                    Bytes::lit(&[0xc4])
                                } else {
                                    Bytes::lit(&[0xc5])
                                }
                            } else {
                                let a1162 = {
                                    let a1160 = v0;
                                    let a1161 = 1u32;
                                    fixed_u32::bit_and(a1160, a1161)
                                };
                                let a1163 = 0u32;
                                if equal(a1162, a1163) {
                                    Bytes::lit(&[0xc6])
                                } else {
                                    Bytes::lit(&[0xc7])
                                }
                            }
                        }
                    } else {
                        let a1166 = {
                            let a1164 = v0;
                            let a1165 = 4u32;
                            fixed_u32::bit_and(a1164, a1165)
                        };
                        let a1167 = 0u32;
                        if equal(a1166, a1167) {
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
                                    Bytes::lit(&[0xc8])
                                } else {
                                    Bytes::lit(&[0xc9])
                                }
                            } else {
                                let a1178 = {
                                    let a1176 = v0;
                                    let a1177 = 1u32;
                                    fixed_u32::bit_and(a1176, a1177)
                                };
                                let a1179 = 0u32;
                                if equal(a1178, a1179) {
                                    Bytes::lit(&[0xca])
                                } else {
                                    Bytes::lit(&[0xcb])
                                }
                            }
                        } else {
                            let a1182 = {
                                let a1180 = v0;
                                let a1181 = 2u32;
                                fixed_u32::bit_and(a1180, a1181)
                            };
                            let a1183 = 0u32;
                            if equal(a1182, a1183) {
                                let a1186 = {
                                    let a1184 = v0;
                                    let a1185 = 1u32;
                                    fixed_u32::bit_and(a1184, a1185)
                                };
                                let a1187 = 0u32;
                                if equal(a1186, a1187) {
                                    Bytes::lit(&[0xcc])
                                } else {
                                    Bytes::lit(&[0xcd])
                                }
                            } else {
                                let a1190 = {
                                    let a1188 = v0;
                                    let a1189 = 1u32;
                                    fixed_u32::bit_and(a1188, a1189)
                                };
                                let a1191 = 0u32;
                                if equal(a1190, a1191) {
                                    Bytes::lit(&[0xce])
                                } else {
                                    Bytes::lit(&[0xcf])
                                }
                            }
                        }
                    }
                } else {
                    let a1194 = {
                        let a1192 = v0;
                        let a1193 = 8u32;
                        fixed_u32::bit_and(a1192, a1193)
                    };
                    let a1195 = 0u32;
                    if equal(a1194, a1195) {
                        let a1198 = {
                            let a1196 = v0;
                            let a1197 = 4u32;
                            fixed_u32::bit_and(a1196, a1197)
                        };
                        let a1199 = 0u32;
                        if equal(a1198, a1199) {
                            let a1202 = {
                                let a1200 = v0;
                                let a1201 = 2u32;
                                fixed_u32::bit_and(a1200, a1201)
                            };
                            let a1203 = 0u32;
                            if equal(a1202, a1203) {
                                let a1206 = {
                                    let a1204 = v0;
                                    let a1205 = 1u32;
                                    fixed_u32::bit_and(a1204, a1205)
                                };
                                let a1207 = 0u32;
                                if equal(a1206, a1207) {
                                    Bytes::lit(&[0xd0])
                                } else {
                                    Bytes::lit(&[0xd1])
                                }
                            } else {
                                let a1210 = {
                                    let a1208 = v0;
                                    let a1209 = 1u32;
                                    fixed_u32::bit_and(a1208, a1209)
                                };
                                let a1211 = 0u32;
                                if equal(a1210, a1211) {
                                    Bytes::lit(&[0xd2])
                                } else {
                                    Bytes::lit(&[0xd3])
                                }
                            }
                        } else {
                            let a1214 = {
                                let a1212 = v0;
                                let a1213 = 2u32;
                                fixed_u32::bit_and(a1212, a1213)
                            };
                            let a1215 = 0u32;
                            if equal(a1214, a1215) {
                                let a1218 = {
                                    let a1216 = v0;
                                    let a1217 = 1u32;
                                    fixed_u32::bit_and(a1216, a1217)
                                };
                                let a1219 = 0u32;
                                if equal(a1218, a1219) {
                                    Bytes::lit(&[0xd4])
                                } else {
                                    Bytes::lit(&[0xd5])
                                }
                            } else {
                                let a1222 = {
                                    let a1220 = v0;
                                    let a1221 = 1u32;
                                    fixed_u32::bit_and(a1220, a1221)
                                };
                                let a1223 = 0u32;
                                if equal(a1222, a1223) {
                                    Bytes::lit(&[0xd6])
                                } else {
                                    Bytes::lit(&[0xd7])
                                }
                            }
                        }
                    } else {
                        let a1226 = {
                            let a1224 = v0;
                            let a1225 = 4u32;
                            fixed_u32::bit_and(a1224, a1225)
                        };
                        let a1227 = 0u32;
                        if equal(a1226, a1227) {
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
                                    Bytes::lit(&[0xd8])
                                } else {
                                    Bytes::lit(&[0xd9])
                                }
                            } else {
                                let a1238 = {
                                    let a1236 = v0;
                                    let a1237 = 1u32;
                                    fixed_u32::bit_and(a1236, a1237)
                                };
                                let a1239 = 0u32;
                                if equal(a1238, a1239) {
                                    Bytes::lit(&[0xda])
                                } else {
                                    Bytes::lit(&[0xdb])
                                }
                            }
                        } else {
                            let a1242 = {
                                let a1240 = v0;
                                let a1241 = 2u32;
                                fixed_u32::bit_and(a1240, a1241)
                            };
                            let a1243 = 0u32;
                            if equal(a1242, a1243) {
                                let a1246 = {
                                    let a1244 = v0;
                                    let a1245 = 1u32;
                                    fixed_u32::bit_and(a1244, a1245)
                                };
                                let a1247 = 0u32;
                                if equal(a1246, a1247) {
                                    Bytes::lit(&[0xdc])
                                } else {
                                    Bytes::lit(&[0xdd])
                                }
                            } else {
                                let a1250 = {
                                    let a1248 = v0;
                                    let a1249 = 1u32;
                                    fixed_u32::bit_and(a1248, a1249)
                                };
                                let a1251 = 0u32;
                                if equal(a1250, a1251) {
                                    Bytes::lit(&[0xde])
                                } else {
                                    Bytes::lit(&[0xdf])
                                }
                            }
                        }
                    }
                }
            } else {
                let a1254 = {
                    let a1252 = v0;
                    let a1253 = 16u32;
                    fixed_u32::bit_and(a1252, a1253)
                };
                let a1255 = 0u32;
                if equal(a1254, a1255) {
                    let a1258 = {
                        let a1256 = v0;
                        let a1257 = 8u32;
                        fixed_u32::bit_and(a1256, a1257)
                    };
                    let a1259 = 0u32;
                    if equal(a1258, a1259) {
                        let a1262 = {
                            let a1260 = v0;
                            let a1261 = 4u32;
                            fixed_u32::bit_and(a1260, a1261)
                        };
                        let a1263 = 0u32;
                        if equal(a1262, a1263) {
                            let a1266 = {
                                let a1264 = v0;
                                let a1265 = 2u32;
                                fixed_u32::bit_and(a1264, a1265)
                            };
                            let a1267 = 0u32;
                            if equal(a1266, a1267) {
                                let a1270 = {
                                    let a1268 = v0;
                                    let a1269 = 1u32;
                                    fixed_u32::bit_and(a1268, a1269)
                                };
                                let a1271 = 0u32;
                                if equal(a1270, a1271) {
                                    Bytes::lit(&[0xe0])
                                } else {
                                    Bytes::lit(&[0xe1])
                                }
                            } else {
                                let a1274 = {
                                    let a1272 = v0;
                                    let a1273 = 1u32;
                                    fixed_u32::bit_and(a1272, a1273)
                                };
                                let a1275 = 0u32;
                                if equal(a1274, a1275) {
                                    Bytes::lit(&[0xe2])
                                } else {
                                    Bytes::lit(&[0xe3])
                                }
                            }
                        } else {
                            let a1278 = {
                                let a1276 = v0;
                                let a1277 = 2u32;
                                fixed_u32::bit_and(a1276, a1277)
                            };
                            let a1279 = 0u32;
                            if equal(a1278, a1279) {
                                let a1282 = {
                                    let a1280 = v0;
                                    let a1281 = 1u32;
                                    fixed_u32::bit_and(a1280, a1281)
                                };
                                let a1283 = 0u32;
                                if equal(a1282, a1283) {
                                    Bytes::lit(&[0xe4])
                                } else {
                                    Bytes::lit(&[0xe5])
                                }
                            } else {
                                let a1286 = {
                                    let a1284 = v0;
                                    let a1285 = 1u32;
                                    fixed_u32::bit_and(a1284, a1285)
                                };
                                let a1287 = 0u32;
                                if equal(a1286, a1287) {
                                    Bytes::lit(&[0xe6])
                                } else {
                                    Bytes::lit(&[0xe7])
                                }
                            }
                        }
                    } else {
                        let a1290 = {
                            let a1288 = v0;
                            let a1289 = 4u32;
                            fixed_u32::bit_and(a1288, a1289)
                        };
                        let a1291 = 0u32;
                        if equal(a1290, a1291) {
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
                                    Bytes::lit(&[0xe8])
                                } else {
                                    Bytes::lit(&[0xe9])
                                }
                            } else {
                                let a1302 = {
                                    let a1300 = v0;
                                    let a1301 = 1u32;
                                    fixed_u32::bit_and(a1300, a1301)
                                };
                                let a1303 = 0u32;
                                if equal(a1302, a1303) {
                                    Bytes::lit(&[0xea])
                                } else {
                                    Bytes::lit(&[0xeb])
                                }
                            }
                        } else {
                            let a1306 = {
                                let a1304 = v0;
                                let a1305 = 2u32;
                                fixed_u32::bit_and(a1304, a1305)
                            };
                            let a1307 = 0u32;
                            if equal(a1306, a1307) {
                                let a1310 = {
                                    let a1308 = v0;
                                    let a1309 = 1u32;
                                    fixed_u32::bit_and(a1308, a1309)
                                };
                                let a1311 = 0u32;
                                if equal(a1310, a1311) {
                                    Bytes::lit(&[0xec])
                                } else {
                                    Bytes::lit(&[0xed])
                                }
                            } else {
                                let a1314 = {
                                    let a1312 = v0;
                                    let a1313 = 1u32;
                                    fixed_u32::bit_and(a1312, a1313)
                                };
                                let a1315 = 0u32;
                                if equal(a1314, a1315) {
                                    Bytes::lit(&[0xee])
                                } else {
                                    Bytes::lit(&[0xef])
                                }
                            }
                        }
                    }
                } else {
                    let a1318 = {
                        let a1316 = v0;
                        let a1317 = 8u32;
                        fixed_u32::bit_and(a1316, a1317)
                    };
                    let a1319 = 0u32;
                    if equal(a1318, a1319) {
                        let a1322 = {
                            let a1320 = v0;
                            let a1321 = 4u32;
                            fixed_u32::bit_and(a1320, a1321)
                        };
                        let a1323 = 0u32;
                        if equal(a1322, a1323) {
                            let a1326 = {
                                let a1324 = v0;
                                let a1325 = 2u32;
                                fixed_u32::bit_and(a1324, a1325)
                            };
                            let a1327 = 0u32;
                            if equal(a1326, a1327) {
                                let a1330 = {
                                    let a1328 = v0;
                                    let a1329 = 1u32;
                                    fixed_u32::bit_and(a1328, a1329)
                                };
                                let a1331 = 0u32;
                                if equal(a1330, a1331) {
                                    Bytes::lit(&[0xf0])
                                } else {
                                    Bytes::lit(&[0xf1])
                                }
                            } else {
                                let a1334 = {
                                    let a1332 = v0;
                                    let a1333 = 1u32;
                                    fixed_u32::bit_and(a1332, a1333)
                                };
                                let a1335 = 0u32;
                                if equal(a1334, a1335) {
                                    Bytes::lit(&[0xf2])
                                } else {
                                    Bytes::lit(&[0xf3])
                                }
                            }
                        } else {
                            let a1338 = {
                                let a1336 = v0;
                                let a1337 = 2u32;
                                fixed_u32::bit_and(a1336, a1337)
                            };
                            let a1339 = 0u32;
                            if equal(a1338, a1339) {
                                let a1342 = {
                                    let a1340 = v0;
                                    let a1341 = 1u32;
                                    fixed_u32::bit_and(a1340, a1341)
                                };
                                let a1343 = 0u32;
                                if equal(a1342, a1343) {
                                    Bytes::lit(&[0xf4])
                                } else {
                                    Bytes::lit(&[0xf5])
                                }
                            } else {
                                let a1346 = {
                                    let a1344 = v0;
                                    let a1345 = 1u32;
                                    fixed_u32::bit_and(a1344, a1345)
                                };
                                let a1347 = 0u32;
                                if equal(a1346, a1347) {
                                    Bytes::lit(&[0xf6])
                                } else {
                                    Bytes::lit(&[0xf7])
                                }
                            }
                        }
                    } else {
                        let a1350 = {
                            let a1348 = v0;
                            let a1349 = 4u32;
                            fixed_u32::bit_and(a1348, a1349)
                        };
                        let a1351 = 0u32;
                        if equal(a1350, a1351) {
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
                                    Bytes::lit(&[0xf8])
                                } else {
                                    Bytes::lit(&[0xf9])
                                }
                            } else {
                                let a1362 = {
                                    let a1360 = v0;
                                    let a1361 = 1u32;
                                    fixed_u32::bit_and(a1360, a1361)
                                };
                                let a1363 = 0u32;
                                if equal(a1362, a1363) {
                                    Bytes::lit(&[0xfa])
                                } else {
                                    Bytes::lit(&[0xfb])
                                }
                            }
                        } else {
                            let a1366 = {
                                let a1364 = v0;
                                let a1365 = 2u32;
                                fixed_u32::bit_and(a1364, a1365)
                            };
                            let a1367 = 0u32;
                            if equal(a1366, a1367) {
                                let a1370 = {
                                    let a1368 = v0;
                                    let a1369 = 1u32;
                                    fixed_u32::bit_and(a1368, a1369)
                                };
                                let a1371 = 0u32;
                                if equal(a1370, a1371) {
                                    Bytes::lit(&[0xfc])
                                } else {
                                    Bytes::lit(&[0xfd])
                                }
                            } else {
                                let a1374 = {
                                    let a1372 = v0;
                                    let a1373 = 1u32;
                                    fixed_u32::bit_and(a1372, a1373)
                                };
                                let a1375 = 0u32;
                                if equal(a1374, a1375) {
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

pub fn f44(v0: u32, v1: u32) -> u32 {
    let m1378 = {
        let a1376 = v0;
        let a1377 = v1;
        fixed_u32::shift_right(a1376, a1377)
    };
    match m1378 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f45(v0: u32, v1: u32, v2: u32, v3: u32) -> u32 {
    let a1383 = v0;
    let a1384 = {
        let a1381 = f47(v1, 8u32);
        let a1382 = {
            let a1379 = f47(v2, 16u32);
            let a1380 = f47(v3, 24u32);
            fixed_u32::bit_or(a1379, a1380)
        };
        fixed_u32::bit_or(a1381, a1382)
    };
    fixed_u32::bit_or(a1383, a1384)
}

pub fn f46(v0: Bytes, v1: u64, v2: u64) -> u32 {
    let a1385 = v2;
    let a1386 = v1;
    if nat_lt(a1385, a1386) {
        let m1389 = {
            let a1387 = v0;
            let a1388 = v2;
            index_bytes(a1387, a1388)
        };
        match m1389 {
            None => {
                0u32
            }
            Some(v3) => {
                let m1391 = {
                    let a1390 = v3;
                    fixed_u32::convert(i128::from(a1390))
                };
                match m1391 {
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

pub fn f47(v0: u32, v1: u32) -> u32 {
    let m1394 = {
        let a1392 = v0;
        let a1393 = v1;
        fixed_u32::shift_left(a1392, a1393)
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

pub fn f48(v0: Adt3, v1: Adt3) -> Adt3 {
    f50(f51(f52(f53(f54(f55(f56(f57(v0, v1.clone()), v1.clone()), v1.clone()), v1.clone()), v1.clone()), v1.clone()), v1.clone()), v1)
}

pub fn f49(v0: Adt3) -> Adt3 {
    Adt3::C0(match v0.clone() {
        Adt3::C0(_, _, h1395, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1395
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h1396, _, _, _, _, _, _, _, _, _) => {
            h1396
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h1397, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1397
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h1398, _, _, _, _, _) => {
            h1398
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, h1399, _, _, _, _, _, _, _, _) => {
            h1399
        }
    }, match v0.clone() {
        Adt3::C0(h1400, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1400
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h1401, _, _, _, _, _, _, _, _, _, _, _) => {
            h1401
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1402, _, _) => {
            h1402
        }
    }, match v0.clone() {
        Adt3::C0(_, h1403, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1403
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h1404, _, _, _, _) => {
            h1404
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1405, _, _, _) => {
            h1405
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h1406, _, _, _, _, _, _, _, _, _, _) => {
            h1406
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h1407, _, _, _, _, _, _) => {
            h1407
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1408, _) => {
            h1408
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1409) => {
            h1409
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, _, h1410, _, _, _, _, _, _, _) => {
            h1410
        }
    })
}

pub fn f50(v0: Adt3, v1: Adt3) -> Adt3 {
    let v2: Adt4 = f58(match v0.clone() {
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

pub fn f51(v0: Adt3, v1: Adt3) -> Adt3 {
    let v2: Adt4 = f58(match v0.clone() {
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

pub fn f52(v0: Adt3, v1: Adt3) -> Adt3 {
    let v2: Adt4 = f58(match v0.clone() {
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

pub fn f53(v0: Adt3, v1: Adt3) -> Adt3 {
    let v2: Adt4 = f58(match v0.clone() {
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

pub fn f54(v0: Adt3, v1: Adt3) -> Adt3 {
    let v2: Adt4 = f58(match v0.clone() {
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

pub fn f55(v0: Adt3, v1: Adt3) -> Adt3 {
    let v2: Adt4 = f58(match v0.clone() {
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

pub fn f56(v0: Adt3, v1: Adt3) -> Adt3 {
    let v2: Adt4 = f58(match v0.clone() {
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

pub fn f57(v0: Adt3, v1: Adt3) -> Adt3 {
    let v2: Adt4 = f58(match v0.clone() {
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

pub fn f58(v0: u32, v1: u32, v2: u32, v3: u32, v4: u32, v5: u32) -> Adt4 {
    let v6: u32 = f59(f59(v0, v1), v4);
    let v7: u32 = f60(f42(v3, v6), 16u32, 16u32);
    let v8: u32 = f59(v2, v7);
    let v9: u32 = f60(f42(v1, v8), 12u32, 20u32);
    let v10: u32 = f59(f59(v6, v9), v5);
    let v11: u32 = f60(f42(v7, v10), 8u32, 24u32);
    let v12: u32 = f59(v8, v11);
    let v13: u32 = f60(f42(v9, v12), 7u32, 25u32);
    Adt4::C0(v10, v13, v12, v11)
}

pub fn f59(v0: u32, v1: u32) -> u32 {
    let m1589 = {
        let a1587 = f61(v0);
        let a1588 = f61(v1);
        fixed_u64::checked_add(a1587, a1588)
    };
    match m1589 {
        None => {
            0u32
        }
        Some(v2) => {
            f62(v2)
        }
    }
}

pub fn f60(v0: u32, v1: u32, v2: u32) -> u32 {
    let a1590 = f44(v0, v1);
    let a1591 = f47(v0, v2);
    fixed_u32::bit_or(a1590, a1591)
}

pub fn f61(v0: u32) -> u64 {
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

pub fn f62(v0: u64) -> u32 {
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

pub fn run(p0: Bytes, p1: u64) -> R<Bytes> {
    f0(p0, p1)
}
