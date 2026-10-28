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
    F4(Bytes),
    F17(Adt2),
}

impl Fn0 {
    pub fn apply(&self, p0: (u64, Bytes)) -> R<(u64, Bytes)> {
        match self {
            Fn0::F4(k0) => f4(k0.clone(), p0),
            Fn0::F17(k0) => f17(k0.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn1 {
    F13(u32, Bytes, Adt1, Adt0),
}

impl Fn1 {
    pub fn apply(&self, p0: (u64, List<Adt1>)) -> R<(u64, List<Adt1>)> {
        match self {
            Fn1::F13(k0, k1, k2, k3) => f13(*k0, k1.clone(), k2.clone(), k3.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn2 {
    F19(u32, Adt1),
}

impl Fn2 {
    pub fn apply(&self, p0: Adt2, p1: Adt1) -> Adt2 {
        match self {
            Fn2::F19(k0, k1) => f19(*k0, k1.clone(), p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn3 {
    F30(u32, u64, u32, u64, u32, Bytes),
}

impl Fn3 {
    pub fn apply(&self, p0: (u64, Adt1)) -> R<(u64, Adt1)> {
        match self {
            Fn3::F30(k0, k1, k2, k3, k4, k5) => f30(*k0, *k1, *k2, *k3, *k4, k5.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn4 {
    F36(u32, Adt1),
}

impl Fn4 {
    pub fn apply(&self, p0: (Adt1, (u64, List<Adt1>))) -> Option<(Adt1, (u64, List<Adt1>))> {
        match self {
            Fn4::F36(k0, k1) => f36(*k0, k1.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn5 {
    F44,
}

impl Fn5 {
    pub fn apply(&self, p0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
        match self {
            Fn5::F44 => f44(p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn6 {
    F48,
}

impl Fn6 {
    pub fn apply(&self, p0: (Adt3, Adt3)) -> (Adt3, Adt3) {
        match self {
            Fn6::F48 => f48(p0),
        }
    }
}

pub fn f0(v0: Bytes) -> R<Bytes> {
    f1(f2(v0, 32u64)?)
}

pub fn f1(v0: Bytes) -> R<Bytes> {
    let (_, h1) = f3(Fn0::F4(v0.clone()), {
        let a2 = v0;
        length_bytes(a2)
    }, (0u64, Bytes::lit(&[])))?;
    Ok(h1)
}

pub fn f2(v0: Bytes, v1: u64) -> R<Bytes> {
    f5(f6(f7(f8(), 0u32), v0)?, v1)
}

pub fn f3(v0: Fn0, p6: u64, p7: (u64, Bytes)) -> R<(u64, Bytes)> {
    let m5 = (p6, p7);
    {
        let mut m5 = m5;
        loop {
            let step = {
                let (v1, v2) = m5;
                let m3 = v1;
                if m3 == 0 {
                    Err::<(u64, (u64, Bytes)), R<(u64, Bytes)>>(Ok(v2))
                } else {
                    let v3 = m3 - 1;
                    Ok::<(u64, (u64, Bytes)), R<(u64, Bytes)>>((v3, {
                        let c4: Fn0 = v0.clone();
                        c4.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m5 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f4(v0: Bytes, v1: (u64, Bytes)) -> R<(u64, Bytes)> {
    f9(v0, v1)
}

pub fn f5(v0: Adt0, v1: u64) -> R<Bytes> {
    f10(f11(v0)?, v1)
}

pub fn f6(v0: Adt0, v1: Bytes) -> R<Adt0> {
    let v2: Bytes = {
        let a9 = match v0.clone() {
            Adt0::C0(_, _, _, h8, _) => {
                h8
            }
        };
        let a10 = v1;
        append_bytes(a9, a10)
    };
    let v3: u64 = {
        let a11 = v2.clone();
        length_bytes(a11)
    };
    let v4: u64 = {
        let a12 = v3;
        let a13 = 0u64;
        if nat_eq(a12, a13) {
            0u64
        } else {
            let a16 = {
                let a14 = v3;
                let a15 = 1u64;
                nat_sub(a14, a15)
            };
            let a17 = 1024u64;
            let a18 = 0u64;
            nat_quot(a16, a17, a18)
        }
    };
    let v5: u32 = match v0.clone() {
        Adt0::C0(_, h19, _, _, _) => {
            h19
        }
    };
    let v6: Adt1 = match v0.clone() {
        Adt0::C0(h20, _, _, _, _) => {
            h20
        }
    };
    let v7: (u64, List<Adt1>) = f12(Fn1::F13(v5, v2.clone(), v6.clone(), v0.clone()), v4, (0u64, match v0.clone() {
        Adt0::C0(_, _, _, _, h21) => {
            h21
        }
    }))?;
    let v8: u64 = {
        let a22 = v4;
        let a23 = 1024u64;
        nat_mul(a22, a23)?
    };
    Ok(Adt0::C0(v6, v5, {
        let a25 = match v0 {
            Adt0::C0(_, _, h24, _, _) => {
                h24
            }
        };
        let a26 = v4;
        nat_add(a25, a26)?
    }, {
        let m32 = {
            let a29 = v2;
            let a30 = v8;
            let a31 = {
                let a27 = v3;
                let a28 = v8;
                nat_sub(a27, a28)
            };
            slice_bytes(a29, a30, a31)
        };
        match m32 {
            None => {
                Bytes::lit(&[])
            }
            Some(v9) => {
                v9
            }
        }
    }, {
        let (_, h33) = v7;
        h33
    }))
}

pub fn f7(v0: Adt1, v1: u32) -> Adt0 {
    Adt0::C0(v0, v1, 0u64, Bytes::lit(&[]), List::<Adt1>::nil())
}

pub fn f8() -> Adt1 {
    Adt1::C0(1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, 1359893119u32, 2600822924u32, 528734635u32, 1541459225u32)
}

pub fn f9(v0: Bytes, v1: (u64, Bytes)) -> R<(u64, Bytes)> {
    let v2: u64 = {
        let (h34, _) = v1;
        h34
    };
    let v3: u32 = f14(v0.clone(), {
        let a35 = v0;
        length_bytes(a35)
    }, v2);
    Ok(({
        let a36 = v2;
        let a37 = 1u64;
        nat_add(a36, a37)?
    }, {
        let a43 = {
            let (_, h38) = v1;
            h38
        };
        let a44 = {
            let a41 = f15(f16(v3, 4u32));
            let a42 = f15({
                let a39 = v3;
                let a40 = 15u32;
                fixed_u32::bit_and(a39, a40)
            });
            append_bytes(a41, a42)
        };
        append_bytes(a43, a44)
    }))
}

pub fn f10(v0: Adt2, v1: u64) -> R<Bytes> {
    let v2: u64 = {
        let a47 = {
            let a45 = v1;
            let a46 = 63u64;
            nat_add(a45, a46)?
        };
        let a48 = 64u64;
        let a49 = 0u64;
        nat_quot(a47, a48, a49)
    };
    let v3: (u64, Bytes) = f3(Fn0::F17(v0), v2, (0u64, Bytes::lit(&[])))?;
    let m54 = {
        let a51 = {
            let (_, h50) = v3.clone();
            h50
        };
        let a52 = 0u64;
        let a53 = v1;
        slice_bytes(a51, a52, a53)
    };
    match m54 {
        None => {
            let (_, h55) = v3;
            Ok(h55)
        }
        Some(v4) => {
            Ok(v4)
        }
    }
}

pub fn f11(v0: Adt0) -> R<Adt2> {
    let v1: Adt1 = match v0.clone() {
        Adt0::C0(h56, _, _, _, _) => {
            h56
        }
    };
    let v2: u32 = match v0.clone() {
        Adt0::C0(_, h57, _, _, _) => {
            h57
        }
    };
    let v3: Bytes = match v0.clone() {
        Adt0::C0(_, _, _, h58, _) => {
            h58
        }
    };
    Ok(f18(Fn2::F19(v2, v1.clone()), f20(v1, v2, v3.clone(), 0u64, {
        let a59 = v3;
        length_bytes(a59)
    }, match v0.clone() {
        Adt0::C0(_, _, h60, _, _) => {
            h60
        }
    })?, match v0 {
        Adt0::C0(_, _, _, _, h61) => {
            h61
        }
    }))
}

pub fn f12(v0: Fn1, p65: u64, p66: (u64, List<Adt1>)) -> R<(u64, List<Adt1>)> {
    let m64 = (p65, p66);
    {
        let mut m64 = m64;
        loop {
            let step = {
                let (v1, v2) = m64;
                let m62 = v1;
                if m62 == 0 {
                    Err::<(u64, (u64, List<Adt1>)), R<(u64, List<Adt1>)>>(Ok(v2))
                } else {
                    let v3 = m62 - 1;
                    Ok::<(u64, (u64, List<Adt1>)), R<(u64, List<Adt1>)>>((v3, {
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

pub fn f13(v0: u32, v1: Bytes, v2: Adt1, v3: Adt0, v4: (u64, List<Adt1>)) -> R<(u64, List<Adt1>)> {
    let v5: u64 = {
        let a69 = match v3 {
            Adt0::C0(_, _, h67, _, _) => {
                h67
            }
        };
        let a70 = {
            let (h68, _) = v4.clone();
            h68
        };
        nat_add(a69, a70)?
    };
    Ok(({
        let a72 = {
            let (h71, _) = v4.clone();
            h71
        };
        let a73 = 1u64;
        nat_add(a72, a73)?
    }, f21(v2.clone(), v0, {
        let (_, h74) = v4;
        h74
    }, f22(f20(v2, v0, v1, {
        let a76 = {
            let (h75, _) = v4;
            h75
        };
        let a77 = 1024u64;
        nat_mul(a76, a77)?
    }, 1024u64, v5)?), {
        let a78 = v5;
        let a79 = 1u64;
        nat_add(a78, a79)?
    })))
}

pub fn f14(v0: Bytes, v1: u64, v2: u64) -> u32 {
    let a80 = v2;
    let a81 = v1;
    if nat_lt(a80, a81) {
        let m84 = {
            let a82 = v0;
            let a83 = v2;
            index_bytes(a82, a83)
        };
        match m84 {
            None => {
                0u32
            }
            Some(v3) => {
                let m86 = {
                    let a85 = v3;
                    fixed_u32::convert(i128::from(a85))
                };
                match m86 {
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

pub fn f15(v0: u32) -> Bytes {
    let a89 = {
        let a87 = v0;
        let a88 = 8u32;
        fixed_u32::bit_and(a87, a88)
    };
    let a90 = 0u32;
    if equal(a89, a90) {
        let a93 = {
            let a91 = v0;
            let a92 = 4u32;
            fixed_u32::bit_and(a91, a92)
        };
        let a94 = 0u32;
        if equal(a93, a94) {
            let a97 = {
                let a95 = v0;
                let a96 = 2u32;
                fixed_u32::bit_and(a95, a96)
            };
            let a98 = 0u32;
            if equal(a97, a98) {
                let a101 = {
                    let a99 = v0;
                    let a100 = 1u32;
                    fixed_u32::bit_and(a99, a100)
                };
                let a102 = 0u32;
                if equal(a101, a102) {
                    Bytes::lit(&[0x30])
                } else {
                    Bytes::lit(&[0x31])
                }
            } else {
                let a105 = {
                    let a103 = v0;
                    let a104 = 1u32;
                    fixed_u32::bit_and(a103, a104)
                };
                let a106 = 0u32;
                if equal(a105, a106) {
                    Bytes::lit(&[0x32])
                } else {
                    Bytes::lit(&[0x33])
                }
            }
        } else {
            let a109 = {
                let a107 = v0;
                let a108 = 2u32;
                fixed_u32::bit_and(a107, a108)
            };
            let a110 = 0u32;
            if equal(a109, a110) {
                let a113 = {
                    let a111 = v0;
                    let a112 = 1u32;
                    fixed_u32::bit_and(a111, a112)
                };
                let a114 = 0u32;
                if equal(a113, a114) {
                    Bytes::lit(&[0x34])
                } else {
                    Bytes::lit(&[0x35])
                }
            } else {
                let a117 = {
                    let a115 = v0;
                    let a116 = 1u32;
                    fixed_u32::bit_and(a115, a116)
                };
                let a118 = 0u32;
                if equal(a117, a118) {
                    Bytes::lit(&[0x36])
                } else {
                    Bytes::lit(&[0x37])
                }
            }
        }
    } else {
        let a121 = {
            let a119 = v0;
            let a120 = 4u32;
            fixed_u32::bit_and(a119, a120)
        };
        let a122 = 0u32;
        if equal(a121, a122) {
            let a125 = {
                let a123 = v0;
                let a124 = 2u32;
                fixed_u32::bit_and(a123, a124)
            };
            let a126 = 0u32;
            if equal(a125, a126) {
                let a129 = {
                    let a127 = v0;
                    let a128 = 1u32;
                    fixed_u32::bit_and(a127, a128)
                };
                let a130 = 0u32;
                if equal(a129, a130) {
                    Bytes::lit(&[0x38])
                } else {
                    Bytes::lit(&[0x39])
                }
            } else {
                let a133 = {
                    let a131 = v0;
                    let a132 = 1u32;
                    fixed_u32::bit_and(a131, a132)
                };
                let a134 = 0u32;
                if equal(a133, a134) {
                    Bytes::lit(&[0x61])
                } else {
                    Bytes::lit(&[0x62])
                }
            }
        } else {
            let a137 = {
                let a135 = v0;
                let a136 = 2u32;
                fixed_u32::bit_and(a135, a136)
            };
            let a138 = 0u32;
            if equal(a137, a138) {
                let a141 = {
                    let a139 = v0;
                    let a140 = 1u32;
                    fixed_u32::bit_and(a139, a140)
                };
                let a142 = 0u32;
                if equal(a141, a142) {
                    Bytes::lit(&[0x63])
                } else {
                    Bytes::lit(&[0x64])
                }
            } else {
                let a145 = {
                    let a143 = v0;
                    let a144 = 1u32;
                    fixed_u32::bit_and(a143, a144)
                };
                let a146 = 0u32;
                if equal(a145, a146) {
                    Bytes::lit(&[0x65])
                } else {
                    Bytes::lit(&[0x66])
                }
            }
        }
    }
}

pub fn f16(v0: u32, v1: u32) -> u32 {
    let m149 = {
        let a147 = v0;
        let a148 = v1;
        fixed_u32::shift_right(a147, a148)
    };
    match m149 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f17(v0: Adt2, v1: (u64, Bytes)) -> R<(u64, Bytes)> {
    Ok(({
        let a151 = {
            let (h150, _) = v1.clone();
            h150
        };
        let a152 = 1u64;
        nat_add(a151, a152)?
    }, {
        let a155 = {
            let (_, h153) = v1;
            h153
        };
        let a156 = f23(f24(v0, {
            let (h154, _) = v1;
            h154
        }));
        append_bytes(a155, a156)
    }))
}

pub fn f18(v0: Fn2, p160: Adt2, p161: List<Adt1>) -> Adt2 {
    let m159 = (p160, p161);
    {
        let mut m159 = m159;
        loop {
            let step = {
                let (v1, v2) = m159;
                let m157 = v2;
                match m157.uncons() {
                    None => {
                        Err::<(Adt2, List<Adt1>), Adt2>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt2, List<Adt1>), Adt2>(({
                            let c158: Fn2 = v0.clone();
                            c158.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m159 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f19(v0: u32, v1: Adt1, v2: Adt2, v3: Adt1) -> Adt2 {
    f25(v3, f22(v2), v1, v0)
}

pub fn f20(v0: Adt1, v1: u32, v2: Bytes, v3: u64, v4: u64, v5: u64) -> R<Adt2> {
    let v6: u64 = {
        let a162 = v3;
        let a163 = v4;
        nat_add(a162, a163)?
    };
    let v7: u64 = f26(v4)?;
    let v8: u32 = f27(v5);
    let v9: u32 = f28(v5);
    let v10: (u64, Adt1) = f29(Fn3::F30(v1, v3, v9, v6, v8, v2.clone()), {
        let a164 = v7;
        let a165 = 64u64;
        let a166 = 0u64;
        nat_quot(a164, a165, a166)
    }, (0u64, v0))?;
    Ok(Adt2::C0({
        let (_, h167) = v10;
        h167
    }, f31(v2, v6, {
        let a168 = v3;
        let a169 = v7;
        nat_add(a168, a169)?
    })?, v5, f32({
        let a170 = v4;
        let a171 = v7;
        nat_sub(a170, a171)
    }, 7u64), {
        let a174 = v1;
        let a175 = {
            let a172 = f33();
            let a173 = f34(v7);
            fixed_u32::bit_or(a172, a173)
        };
        fixed_u32::bit_or(a174, a175)
    }))
}

pub fn f21(v0: Adt1, v1: u32, v2: List<Adt1>, v3: Adt1, v4: u64) -> List<Adt1> {
    let v5: (Adt1, (u64, List<Adt1>)) = {
        let (h176, _) = f35(Fn4::F36(v1, v0), 64u64, (v3, (v4, v2)));
        h176
    };
    List::cons({
        let (h177, _) = v5;
        h177
    }, {
        let (_, h178) = {
            let (_, h179) = v5;
            h179
        };
        h178
    })
}

pub fn f22(v0: Adt2) -> Adt1 {
    f37(f38(match v0.clone() {
        Adt2::C0(h180, _, _, _, _) => {
            h180
        }
    }, match v0.clone() {
        Adt2::C0(_, h181, _, _, _) => {
            h181
        }
    }, f27(match v0.clone() {
        Adt2::C0(_, _, h182, _, _) => {
            h182
        }
    }), f28(match v0.clone() {
        Adt2::C0(_, _, h183, _, _) => {
            h183
        }
    }), match v0.clone() {
        Adt2::C0(_, _, _, h184, _) => {
            h184
        }
    }, match v0 {
        Adt2::C0(_, _, _, _, h185) => {
            h185
        }
    }))
}

pub fn f23(v0: Adt3) -> Bytes {
    let a230 = f39(match v0.clone() {
        Adt3::C0(h186, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h186
        }
    });
    let a231 = {
        let a228 = f39(match v0.clone() {
            Adt3::C0(_, h187, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                h187
            }
        });
        let a229 = {
            let a226 = f39(match v0.clone() {
                Adt3::C0(_, _, h188, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                    h188
                }
            });
            let a227 = {
                let a224 = f39(match v0.clone() {
                    Adt3::C0(_, _, _, h189, _, _, _, _, _, _, _, _, _, _, _, _) => {
                        h189
                    }
                });
                let a225 = {
                    let a222 = f39(match v0.clone() {
                        Adt3::C0(_, _, _, _, h190, _, _, _, _, _, _, _, _, _, _, _) => {
                            h190
                        }
                    });
                    let a223 = {
                        let a220 = f39(match v0.clone() {
                            Adt3::C0(_, _, _, _, _, h191, _, _, _, _, _, _, _, _, _, _) => {
                                h191
                            }
                        });
                        let a221 = {
                            let a218 = f39(match v0.clone() {
                                Adt3::C0(_, _, _, _, _, _, h192, _, _, _, _, _, _, _, _, _) => {
                                    h192
                                }
                            });
                            let a219 = {
                                let a216 = f39(match v0.clone() {
                                    Adt3::C0(_, _, _, _, _, _, _, h193, _, _, _, _, _, _, _, _) => {
                                        h193
                                    }
                                });
                                let a217 = {
                                    let a214 = f39(match v0.clone() {
                                        Adt3::C0(_, _, _, _, _, _, _, _, h194, _, _, _, _, _, _, _) => {
                                            h194
                                        }
                                    });
                                    let a215 = {
                                        let a212 = f39(match v0.clone() {
                                            Adt3::C0(_, _, _, _, _, _, _, _, _, h195, _, _, _, _, _, _) => {
                                                h195
                                            }
                                        });
                                        let a213 = {
                                            let a210 = f39(match v0.clone() {
                                                Adt3::C0(_, _, _, _, _, _, _, _, _, _, h196, _, _, _, _, _) => {
                                                    h196
                                                }
                                            });
                                            let a211 = {
                                                let a208 = f39(match v0.clone() {
                                                    Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h197, _, _, _, _) => {
                                                        h197
                                                    }
                                                });
                                                let a209 = {
                                                    let a206 = f39(match v0.clone() {
                                                        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h198, _, _, _) => {
                                                            h198
                                                        }
                                                    });
                                                    let a207 = {
                                                        let a204 = f39(match v0.clone() {
                                                            Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h199, _, _) => {
                                                                h199
                                                            }
                                                        });
                                                        let a205 = {
                                                            let a202 = f39(match v0.clone() {
                                                                Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h200, _) => {
                                                                    h200
                                                                }
                                                            });
                                                            let a203 = f39(match v0 {
                                                                Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h201) => {
                                                                    h201
                                                                }
                                                            });
                                                            append_bytes(a202, a203)
                                                        };
                                                        append_bytes(a204, a205)
                                                    };
                                                    append_bytes(a206, a207)
                                                };
                                                append_bytes(a208, a209)
                                            };
                                            append_bytes(a210, a211)
                                        };
                                        append_bytes(a212, a213)
                                    };
                                    append_bytes(a214, a215)
                                };
                                append_bytes(a216, a217)
                            };
                            append_bytes(a218, a219)
                        };
                        append_bytes(a220, a221)
                    };
                    append_bytes(a222, a223)
                };
                append_bytes(a224, a225)
            };
            append_bytes(a226, a227)
        };
        append_bytes(a228, a229)
    };
    append_bytes(a230, a231)
}

pub fn f24(v0: Adt2, v1: u64) -> Adt3 {
    f38(match v0.clone() {
        Adt2::C0(h232, _, _, _, _) => {
            h232
        }
    }, match v0.clone() {
        Adt2::C0(_, h233, _, _, _) => {
            h233
        }
    }, f27(v1), f28(v1), match v0.clone() {
        Adt2::C0(_, _, _, h234, _) => {
            h234
        }
    }, {
        let a236 = match v0 {
            Adt2::C0(_, _, _, _, h235) => {
                h235
            }
        };
        let a237 = f40();
        fixed_u32::bit_or(a236, a237)
    })
}

pub fn f25(v0: Adt1, v1: Adt1, v2: Adt1, v3: u32) -> Adt2 {
    Adt2::C0(v2, Adt3::C0(match v0.clone() {
        Adt1::C0(h238, _, _, _, _, _, _, _) => {
            h238
        }
    }, match v0.clone() {
        Adt1::C0(_, h239, _, _, _, _, _, _) => {
            h239
        }
    }, match v0.clone() {
        Adt1::C0(_, _, h240, _, _, _, _, _) => {
            h240
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, h241, _, _, _, _) => {
            h241
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, h242, _, _, _) => {
            h242
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, _, h243, _, _) => {
            h243
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, _, _, h244, _) => {
            h244
        }
    }, match v0 {
        Adt1::C0(_, _, _, _, _, _, _, h245) => {
            h245
        }
    }, match v1.clone() {
        Adt1::C0(h246, _, _, _, _, _, _, _) => {
            h246
        }
    }, match v1.clone() {
        Adt1::C0(_, h247, _, _, _, _, _, _) => {
            h247
        }
    }, match v1.clone() {
        Adt1::C0(_, _, h248, _, _, _, _, _) => {
            h248
        }
    }, match v1.clone() {
        Adt1::C0(_, _, _, h249, _, _, _, _) => {
            h249
        }
    }, match v1.clone() {
        Adt1::C0(_, _, _, _, h250, _, _, _) => {
            h250
        }
    }, match v1.clone() {
        Adt1::C0(_, _, _, _, _, h251, _, _) => {
            h251
        }
    }, match v1.clone() {
        Adt1::C0(_, _, _, _, _, _, h252, _) => {
            h252
        }
    }, match v1 {
        Adt1::C0(_, _, _, _, _, _, _, h253) => {
            h253
        }
    }), 0u64, 64u32, {
        let a254 = v3;
        let a255 = f41();
        fixed_u32::bit_or(a254, a255)
    })
}

pub fn f26(v0: u64) -> R<u64> {
    let a256 = v0;
    let a257 = 0u64;
    if nat_eq(a256, a257) {
        Ok(0u64)
    } else {
        let a263 = {
            let a260 = {
                let a258 = v0;
                let a259 = 1u64;
                nat_sub(a258, a259)
            };
            let a261 = 64u64;
            let a262 = 0u64;
            nat_quot(a260, a261, a262)
        };
        let a264 = 64u64;
        nat_mul(a263, a264)
    }
}

pub fn f27(v0: u64) -> u32 {
    f32({
        let a265 = v0;
        let a266 = 4294967296u64;
        let a267 = 0u64;
        nat_rem(a265, a266, a267)
    }, 32u64)
}

pub fn f28(v0: u64) -> u32 {
    let a268 = v0;
    let a269 = 4294967296u64;
    if nat_lt(a268, a269) {
        0u32
    } else {
        f32({
            let a270 = v0;
            let a271 = 4294967296u64;
            let a272 = 0u64;
            nat_quot(a270, a271, a272)
        }, 32u64)
    }
}

pub fn f29(v0: Fn3, p276: u64, p277: (u64, Adt1)) -> R<(u64, Adt1)> {
    let m275 = (p276, p277);
    {
        let mut m275 = m275;
        loop {
            let step = {
                let (v1, v2) = m275;
                let m273 = v1;
                if m273 == 0 {
                    Err::<(u64, (u64, Adt1)), R<(u64, Adt1)>>(Ok(v2))
                } else {
                    let v3 = m273 - 1;
                    Ok::<(u64, (u64, Adt1)), R<(u64, Adt1)>>((v3, {
                        let c274: Fn3 = v0.clone();
                        c274.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m275 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f30(v0: u32, v1: u64, v2: u32, v3: u64, v4: u32, v5: Bytes, v6: (u64, Adt1)) -> R<(u64, Adt1)> {
    let v7: u64 = {
        let (h278, _) = v6;
        h278
    };
    Ok(({
        let a279 = v7;
        let a280 = 64u64;
        nat_add(a279, a280)?
    }, f37(f38({
        let (_, h281) = v6;
        h281
    }, f31(v5, v3, {
        let a282 = v1;
        let a283 = v7;
        nat_add(a282, a283)?
    })?, v4, v2, 64u32, {
        let a284 = v0;
        let a285 = f34(v7);
        fixed_u32::bit_or(a284, a285)
    }))))
}

pub fn f31(v0: Bytes, v1: u64, v2: u64) -> R<Adt3> {
    Ok(Adt3::C0(f42(v0.clone(), v1, {
        let a286 = v2;
        let a287 = 0u64;
        nat_add(a286, a287)?
    })?, f42(v0.clone(), v1, {
        let a288 = v2;
        let a289 = 4u64;
        nat_add(a288, a289)?
    })?, f42(v0.clone(), v1, {
        let a290 = v2;
        let a291 = 8u64;
        nat_add(a290, a291)?
    })?, f42(v0.clone(), v1, {
        let a292 = v2;
        let a293 = 12u64;
        nat_add(a292, a293)?
    })?, f42(v0.clone(), v1, {
        let a294 = v2;
        let a295 = 16u64;
        nat_add(a294, a295)?
    })?, f42(v0.clone(), v1, {
        let a296 = v2;
        let a297 = 20u64;
        nat_add(a296, a297)?
    })?, f42(v0.clone(), v1, {
        let a298 = v2;
        let a299 = 24u64;
        nat_add(a298, a299)?
    })?, f42(v0.clone(), v1, {
        let a300 = v2;
        let a301 = 28u64;
        nat_add(a300, a301)?
    })?, f42(v0.clone(), v1, {
        let a302 = v2;
        let a303 = 32u64;
        nat_add(a302, a303)?
    })?, f42(v0.clone(), v1, {
        let a304 = v2;
        let a305 = 36u64;
        nat_add(a304, a305)?
    })?, f42(v0.clone(), v1, {
        let a306 = v2;
        let a307 = 40u64;
        nat_add(a306, a307)?
    })?, f42(v0.clone(), v1, {
        let a308 = v2;
        let a309 = 44u64;
        nat_add(a308, a309)?
    })?, f42(v0.clone(), v1, {
        let a310 = v2;
        let a311 = 48u64;
        nat_add(a310, a311)?
    })?, f42(v0.clone(), v1, {
        let a312 = v2;
        let a313 = 52u64;
        nat_add(a312, a313)?
    })?, f42(v0.clone(), v1, {
        let a314 = v2;
        let a315 = 56u64;
        nat_add(a314, a315)?
    })?, f42(v0, v1, {
        let a316 = v2;
        let a317 = 60u64;
        nat_add(a316, a317)?
    })?))
}

pub fn f32(v0: u64, v1: u64) -> u32 {
    let (_, h318) = {
        let (_, h319) = f43(Fn5::F44, v1, (v0, (1u32, 0u32)));
        h319
    };
    h318
}

pub fn f33() -> u32 {
    2u32
}

pub fn f34(v0: u64) -> u32 {
    let a320 = v0;
    let a321 = 0u64;
    if nat_eq(a320, a321) {
        f45()
    } else {
        0u32
    }
}

pub fn f35(v0: Fn4, p326: u64, p327: (Adt1, (u64, List<Adt1>))) -> ((Adt1, (u64, List<Adt1>)), bool) {
    let m325 = (p326, p327);
    {
        let mut m325 = m325;
        loop {
            let step = {
                let (v1, v2) = m325;
                let m322 = v1;
                if m322 == 0 {
                    Err::<(u64, (Adt1, (u64, List<Adt1>))), ((Adt1, (u64, List<Adt1>)), bool)>((v2, false))
                } else {
                    let v3 = m322 - 1;
                    let m324 = {
                        let c323: Fn4 = v0.clone();
                        c323.apply(v2.clone())
                    };
                    match m324 {
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
                Ok(next) => { m325 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f36(v0: u32, v1: Adt1, v2: (Adt1, (u64, List<Adt1>))) -> Option<(Adt1, (u64, List<Adt1>))> {
    f46(v1, v0, v2)
}

pub fn f37(v0: Adt3) -> Adt1 {
    Adt1::C0(match v0.clone() {
        Adt3::C0(h328, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h328
        }
    }, match v0.clone() {
        Adt3::C0(_, h329, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h329
        }
    }, match v0.clone() {
        Adt3::C0(_, _, h330, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h330
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h331, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h331
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h332, _, _, _, _, _, _, _, _, _, _, _) => {
            h332
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h333, _, _, _, _, _, _, _, _, _, _) => {
            h333
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h334, _, _, _, _, _, _, _, _, _) => {
            h334
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, h335, _, _, _, _, _, _, _, _) => {
            h335
        }
    })
}

pub fn f38(v0: Adt1, v1: Adt3, v2: u32, v3: u32, v4: u32, v5: u32) -> Adt3 {
    let v6: Adt3 = {
        let (h336, _) = f47(Fn6::F48, 7u64, (Adt3::C0(match v0.clone() {
            Adt1::C0(h337, _, _, _, _, _, _, _) => {
                h337
            }
        }, match v0.clone() {
            Adt1::C0(_, h338, _, _, _, _, _, _) => {
                h338
            }
        }, match v0.clone() {
            Adt1::C0(_, _, h339, _, _, _, _, _) => {
                h339
            }
        }, match v0.clone() {
            Adt1::C0(_, _, _, h340, _, _, _, _) => {
                h340
            }
        }, match v0.clone() {
            Adt1::C0(_, _, _, _, h341, _, _, _) => {
                h341
            }
        }, match v0.clone() {
            Adt1::C0(_, _, _, _, _, h342, _, _) => {
                h342
            }
        }, match v0.clone() {
            Adt1::C0(_, _, _, _, _, _, h343, _) => {
                h343
            }
        }, match v0.clone() {
            Adt1::C0(_, _, _, _, _, _, _, h344) => {
                h344
            }
        }, 1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, v2, v3, v4, v5), v1));
        h336
    };
    Adt3::C0(f49(match v6.clone() {
        Adt3::C0(h345, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h345
        }
    }, match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, h346, _, _, _, _, _, _, _) => {
            h346
        }
    }), f49(match v6.clone() {
        Adt3::C0(_, h347, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h347
        }
    }, match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h348, _, _, _, _, _, _) => {
            h348
        }
    }), f49(match v6.clone() {
        Adt3::C0(_, _, h349, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h349
        }
    }, match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h350, _, _, _, _, _) => {
            h350
        }
    }), f49(match v6.clone() {
        Adt3::C0(_, _, _, h351, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h351
        }
    }, match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h352, _, _, _, _) => {
            h352
        }
    }), f49(match v6.clone() {
        Adt3::C0(_, _, _, _, h353, _, _, _, _, _, _, _, _, _, _, _) => {
            h353
        }
    }, match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h354, _, _, _) => {
            h354
        }
    }), f49(match v6.clone() {
        Adt3::C0(_, _, _, _, _, h355, _, _, _, _, _, _, _, _, _, _) => {
            h355
        }
    }, match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h356, _, _) => {
            h356
        }
    }), f49(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, h357, _, _, _, _, _, _, _, _, _) => {
            h357
        }
    }, match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h358, _) => {
            h358
        }
    }), f49(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, h359, _, _, _, _, _, _, _, _) => {
            h359
        }
    }, match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h360) => {
            h360
        }
    }), f49(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, h361, _, _, _, _, _, _, _) => {
            h361
        }
    }, match v0.clone() {
        Adt1::C0(h362, _, _, _, _, _, _, _) => {
            h362
        }
    }), f49(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h363, _, _, _, _, _, _) => {
            h363
        }
    }, match v0.clone() {
        Adt1::C0(_, h364, _, _, _, _, _, _) => {
            h364
        }
    }), f49(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h365, _, _, _, _, _) => {
            h365
        }
    }, match v0.clone() {
        Adt1::C0(_, _, h366, _, _, _, _, _) => {
            h366
        }
    }), f49(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h367, _, _, _, _) => {
            h367
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, h368, _, _, _, _) => {
            h368
        }
    }), f49(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h369, _, _, _) => {
            h369
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, h370, _, _, _) => {
            h370
        }
    }), f49(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h371, _, _) => {
            h371
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, _, h372, _, _) => {
            h372
        }
    }), f49(match v6.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h373, _) => {
            h373
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, _, _, h374, _) => {
            h374
        }
    }), f49(match v6 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h375) => {
            h375
        }
    }, match v0 {
        Adt1::C0(_, _, _, _, _, _, _, h376) => {
            h376
        }
    }))
}

pub fn f39(v0: u32) -> Bytes {
    let a387 = f50({
        let a377 = v0;
        let a378 = 255u32;
        fixed_u32::bit_and(a377, a378)
    });
    let a388 = {
        let a385 = f50({
            let a379 = f16(v0, 8u32);
            let a380 = 255u32;
            fixed_u32::bit_and(a379, a380)
        });
        let a386 = {
            let a383 = f50({
                let a381 = f16(v0, 16u32);
                let a382 = 255u32;
                fixed_u32::bit_and(a381, a382)
            });
            let a384 = f50(f16(v0, 24u32));
            append_bytes(a383, a384)
        };
        append_bytes(a385, a386)
    };
    append_bytes(a387, a388)
}

pub fn f40() -> u32 {
    8u32
}

pub fn f41() -> u32 {
    4u32
}

pub fn f42(v0: Bytes, v1: u64, v2: u64) -> R<u32> {
    Ok(f51(f14(v0.clone(), v1, v2), f14(v0.clone(), v1, {
        let a389 = v2;
        let a390 = 1u64;
        nat_add(a389, a390)?
    }), f14(v0.clone(), v1, {
        let a391 = v2;
        let a392 = 2u64;
        nat_add(a391, a392)?
    }), f14(v0, v1, {
        let a393 = v2;
        let a394 = 3u64;
        nat_add(a393, a394)?
    })))
}

pub fn f43(v0: Fn5, p398: u64, p399: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let m397 = (p398, p399);
    {
        let mut m397 = m397;
        loop {
            let step = {
                let (v1, v2) = m397;
                let m395 = v1;
                if m395 == 0 {
                    Err::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>(v2)
                } else {
                    let v3 = m395 - 1;
                    Ok::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>((v3, {
                        let c396: Fn5 = v0.clone();
                        c396.apply(v2)
                    }))
                }
            };
            match step {
                Ok(next) => { m397 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f44(v0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let v1: u64 = {
        let (h400, _) = v0;
        h400
    };
    let v2: u32 = {
        let (h401, _) = {
            let (_, h402) = v0;
            h402
        };
        h401
    };
    let v3: u32 = {
        let (_, h403) = {
            let (_, h404) = v0;
            h404
        };
        h403
    };
    ({
        let a405 = v1;
        let a406 = 2u64;
        let a407 = 0u64;
        nat_quot(a405, a406, a407)
    }, (f52(v2, 1u32), {
        let a411 = {
            let a408 = v1;
            let a409 = 2u64;
            let a410 = 0u64;
            nat_rem(a408, a409, a410)
        };
        let a412 = 1u64;
        if nat_eq(a411, a412) {
            let a413 = v3;
            let a414 = v2;
            fixed_u32::bit_or(a413, a414)
        } else {
            v3
        }
    }))
}

pub fn f45() -> u32 {
    1u32
}

pub fn f46(v0: Adt1, v1: u32, v2: (Adt1, (u64, List<Adt1>))) -> Option<(Adt1, (u64, List<Adt1>))> {
    let v3: u64 = {
        let (h415, _) = {
            let (_, h416) = v2.clone();
            h416
        };
        h415
    };
    let a420 = {
        let a417 = v3;
        let a418 = 2u64;
        let a419 = 0u64;
        nat_rem(a417, a418, a419)
    };
    let a421 = 0u64;
    if nat_eq(a420, a421) {
        let m424 = {
            let (_, h422) = {
                let (_, h423) = v2;
                h423
            };
            h422
        };
        match m424.uncons() {
            None => {
                None::<(Adt1, (u64, List<Adt1>))>
            }
            Some((v4, v5)) => {
                Some((f22(f25(v4, {
                    let (h425, _) = v2;
                    h425
                }, v0, v1)), ({
                    let a426 = v3;
                    let a427 = 2u64;
                    let a428 = 0u64;
                    nat_quot(a426, a427, a428)
                }, v5)))
            }
        }
    } else {
        None::<(Adt1, (u64, List<Adt1>))>
    }
}

pub fn f47(v0: Fn6, p432: u64, p433: (Adt3, Adt3)) -> (Adt3, Adt3) {
    let m431 = (p432, p433);
    {
        let mut m431 = m431;
        loop {
            let step = {
                let (v1, v2) = m431;
                let m429 = v1;
                if m429 == 0 {
                    Err::<(u64, (Adt3, Adt3)), (Adt3, Adt3)>(v2)
                } else {
                    let v3 = m429 - 1;
                    Ok::<(u64, (Adt3, Adt3)), (Adt3, Adt3)>((v3, {
                        let c430: Fn6 = v0.clone();
                        c430.apply(v2)
                    }))
                }
            };
            match step {
                Ok(next) => { m431 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f48(v0: (Adt3, Adt3)) -> (Adt3, Adt3) {
    (f53({
        let (h434, _) = v0.clone();
        h434
    }, {
        let (_, h435) = v0.clone();
        h435
    }), f54({
        let (_, h436) = v0;
        h436
    }))
}

pub fn f49(v0: u32, v1: u32) -> u32 {
    let a437 = v0;
    let a438 = v1;
    fixed_u32::bit_xor(a437, a438)
}

pub fn f50(v0: u32) -> Bytes {
    let a441 = {
        let a439 = v0;
        let a440 = 128u32;
        fixed_u32::bit_and(a439, a440)
    };
    let a442 = 0u32;
    if equal(a441, a442) {
        let a445 = {
            let a443 = v0;
            let a444 = 64u32;
            fixed_u32::bit_and(a443, a444)
        };
        let a446 = 0u32;
        if equal(a445, a446) {
            let a449 = {
                let a447 = v0;
                let a448 = 32u32;
                fixed_u32::bit_and(a447, a448)
            };
            let a450 = 0u32;
            if equal(a449, a450) {
                let a453 = {
                    let a451 = v0;
                    let a452 = 16u32;
                    fixed_u32::bit_and(a451, a452)
                };
                let a454 = 0u32;
                if equal(a453, a454) {
                    let a457 = {
                        let a455 = v0;
                        let a456 = 8u32;
                        fixed_u32::bit_and(a455, a456)
                    };
                    let a458 = 0u32;
                    if equal(a457, a458) {
                        let a461 = {
                            let a459 = v0;
                            let a460 = 4u32;
                            fixed_u32::bit_and(a459, a460)
                        };
                        let a462 = 0u32;
                        if equal(a461, a462) {
                            let a465 = {
                                let a463 = v0;
                                let a464 = 2u32;
                                fixed_u32::bit_and(a463, a464)
                            };
                            let a466 = 0u32;
                            if equal(a465, a466) {
                                let a469 = {
                                    let a467 = v0;
                                    let a468 = 1u32;
                                    fixed_u32::bit_and(a467, a468)
                                };
                                let a470 = 0u32;
                                if equal(a469, a470) {
                                    Bytes::lit(&[0x00])
                                } else {
                                    Bytes::lit(&[0x01])
                                }
                            } else {
                                let a473 = {
                                    let a471 = v0;
                                    let a472 = 1u32;
                                    fixed_u32::bit_and(a471, a472)
                                };
                                let a474 = 0u32;
                                if equal(a473, a474) {
                                    Bytes::lit(&[0x02])
                                } else {
                                    Bytes::lit(&[0x03])
                                }
                            }
                        } else {
                            let a477 = {
                                let a475 = v0;
                                let a476 = 2u32;
                                fixed_u32::bit_and(a475, a476)
                            };
                            let a478 = 0u32;
                            if equal(a477, a478) {
                                let a481 = {
                                    let a479 = v0;
                                    let a480 = 1u32;
                                    fixed_u32::bit_and(a479, a480)
                                };
                                let a482 = 0u32;
                                if equal(a481, a482) {
                                    Bytes::lit(&[0x04])
                                } else {
                                    Bytes::lit(&[0x05])
                                }
                            } else {
                                let a485 = {
                                    let a483 = v0;
                                    let a484 = 1u32;
                                    fixed_u32::bit_and(a483, a484)
                                };
                                let a486 = 0u32;
                                if equal(a485, a486) {
                                    Bytes::lit(&[0x06])
                                } else {
                                    Bytes::lit(&[0x07])
                                }
                            }
                        }
                    } else {
                        let a489 = {
                            let a487 = v0;
                            let a488 = 4u32;
                            fixed_u32::bit_and(a487, a488)
                        };
                        let a490 = 0u32;
                        if equal(a489, a490) {
                            let a493 = {
                                let a491 = v0;
                                let a492 = 2u32;
                                fixed_u32::bit_and(a491, a492)
                            };
                            let a494 = 0u32;
                            if equal(a493, a494) {
                                let a497 = {
                                    let a495 = v0;
                                    let a496 = 1u32;
                                    fixed_u32::bit_and(a495, a496)
                                };
                                let a498 = 0u32;
                                if equal(a497, a498) {
                                    Bytes::lit(&[0x08])
                                } else {
                                    Bytes::lit(&[0x09])
                                }
                            } else {
                                let a501 = {
                                    let a499 = v0;
                                    let a500 = 1u32;
                                    fixed_u32::bit_and(a499, a500)
                                };
                                let a502 = 0u32;
                                if equal(a501, a502) {
                                    Bytes::lit(&[0x0a])
                                } else {
                                    Bytes::lit(&[0x0b])
                                }
                            }
                        } else {
                            let a505 = {
                                let a503 = v0;
                                let a504 = 2u32;
                                fixed_u32::bit_and(a503, a504)
                            };
                            let a506 = 0u32;
                            if equal(a505, a506) {
                                let a509 = {
                                    let a507 = v0;
                                    let a508 = 1u32;
                                    fixed_u32::bit_and(a507, a508)
                                };
                                let a510 = 0u32;
                                if equal(a509, a510) {
                                    Bytes::lit(&[0x0c])
                                } else {
                                    Bytes::lit(&[0x0d])
                                }
                            } else {
                                let a513 = {
                                    let a511 = v0;
                                    let a512 = 1u32;
                                    fixed_u32::bit_and(a511, a512)
                                };
                                let a514 = 0u32;
                                if equal(a513, a514) {
                                    Bytes::lit(&[0x0e])
                                } else {
                                    Bytes::lit(&[0x0f])
                                }
                            }
                        }
                    }
                } else {
                    let a517 = {
                        let a515 = v0;
                        let a516 = 8u32;
                        fixed_u32::bit_and(a515, a516)
                    };
                    let a518 = 0u32;
                    if equal(a517, a518) {
                        let a521 = {
                            let a519 = v0;
                            let a520 = 4u32;
                            fixed_u32::bit_and(a519, a520)
                        };
                        let a522 = 0u32;
                        if equal(a521, a522) {
                            let a525 = {
                                let a523 = v0;
                                let a524 = 2u32;
                                fixed_u32::bit_and(a523, a524)
                            };
                            let a526 = 0u32;
                            if equal(a525, a526) {
                                let a529 = {
                                    let a527 = v0;
                                    let a528 = 1u32;
                                    fixed_u32::bit_and(a527, a528)
                                };
                                let a530 = 0u32;
                                if equal(a529, a530) {
                                    Bytes::lit(&[0x10])
                                } else {
                                    Bytes::lit(&[0x11])
                                }
                            } else {
                                let a533 = {
                                    let a531 = v0;
                                    let a532 = 1u32;
                                    fixed_u32::bit_and(a531, a532)
                                };
                                let a534 = 0u32;
                                if equal(a533, a534) {
                                    Bytes::lit(&[0x12])
                                } else {
                                    Bytes::lit(&[0x13])
                                }
                            }
                        } else {
                            let a537 = {
                                let a535 = v0;
                                let a536 = 2u32;
                                fixed_u32::bit_and(a535, a536)
                            };
                            let a538 = 0u32;
                            if equal(a537, a538) {
                                let a541 = {
                                    let a539 = v0;
                                    let a540 = 1u32;
                                    fixed_u32::bit_and(a539, a540)
                                };
                                let a542 = 0u32;
                                if equal(a541, a542) {
                                    Bytes::lit(&[0x14])
                                } else {
                                    Bytes::lit(&[0x15])
                                }
                            } else {
                                let a545 = {
                                    let a543 = v0;
                                    let a544 = 1u32;
                                    fixed_u32::bit_and(a543, a544)
                                };
                                let a546 = 0u32;
                                if equal(a545, a546) {
                                    Bytes::lit(&[0x16])
                                } else {
                                    Bytes::lit(&[0x17])
                                }
                            }
                        }
                    } else {
                        let a549 = {
                            let a547 = v0;
                            let a548 = 4u32;
                            fixed_u32::bit_and(a547, a548)
                        };
                        let a550 = 0u32;
                        if equal(a549, a550) {
                            let a553 = {
                                let a551 = v0;
                                let a552 = 2u32;
                                fixed_u32::bit_and(a551, a552)
                            };
                            let a554 = 0u32;
                            if equal(a553, a554) {
                                let a557 = {
                                    let a555 = v0;
                                    let a556 = 1u32;
                                    fixed_u32::bit_and(a555, a556)
                                };
                                let a558 = 0u32;
                                if equal(a557, a558) {
                                    Bytes::lit(&[0x18])
                                } else {
                                    Bytes::lit(&[0x19])
                                }
                            } else {
                                let a561 = {
                                    let a559 = v0;
                                    let a560 = 1u32;
                                    fixed_u32::bit_and(a559, a560)
                                };
                                let a562 = 0u32;
                                if equal(a561, a562) {
                                    Bytes::lit(&[0x1a])
                                } else {
                                    Bytes::lit(&[0x1b])
                                }
                            }
                        } else {
                            let a565 = {
                                let a563 = v0;
                                let a564 = 2u32;
                                fixed_u32::bit_and(a563, a564)
                            };
                            let a566 = 0u32;
                            if equal(a565, a566) {
                                let a569 = {
                                    let a567 = v0;
                                    let a568 = 1u32;
                                    fixed_u32::bit_and(a567, a568)
                                };
                                let a570 = 0u32;
                                if equal(a569, a570) {
                                    Bytes::lit(&[0x1c])
                                } else {
                                    Bytes::lit(&[0x1d])
                                }
                            } else {
                                let a573 = {
                                    let a571 = v0;
                                    let a572 = 1u32;
                                    fixed_u32::bit_and(a571, a572)
                                };
                                let a574 = 0u32;
                                if equal(a573, a574) {
                                    Bytes::lit(&[0x1e])
                                } else {
                                    Bytes::lit(&[0x1f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a577 = {
                    let a575 = v0;
                    let a576 = 16u32;
                    fixed_u32::bit_and(a575, a576)
                };
                let a578 = 0u32;
                if equal(a577, a578) {
                    let a581 = {
                        let a579 = v0;
                        let a580 = 8u32;
                        fixed_u32::bit_and(a579, a580)
                    };
                    let a582 = 0u32;
                    if equal(a581, a582) {
                        let a585 = {
                            let a583 = v0;
                            let a584 = 4u32;
                            fixed_u32::bit_and(a583, a584)
                        };
                        let a586 = 0u32;
                        if equal(a585, a586) {
                            let a589 = {
                                let a587 = v0;
                                let a588 = 2u32;
                                fixed_u32::bit_and(a587, a588)
                            };
                            let a590 = 0u32;
                            if equal(a589, a590) {
                                let a593 = {
                                    let a591 = v0;
                                    let a592 = 1u32;
                                    fixed_u32::bit_and(a591, a592)
                                };
                                let a594 = 0u32;
                                if equal(a593, a594) {
                                    Bytes::lit(&[0x20])
                                } else {
                                    Bytes::lit(&[0x21])
                                }
                            } else {
                                let a597 = {
                                    let a595 = v0;
                                    let a596 = 1u32;
                                    fixed_u32::bit_and(a595, a596)
                                };
                                let a598 = 0u32;
                                if equal(a597, a598) {
                                    Bytes::lit(&[0x22])
                                } else {
                                    Bytes::lit(&[0x23])
                                }
                            }
                        } else {
                            let a601 = {
                                let a599 = v0;
                                let a600 = 2u32;
                                fixed_u32::bit_and(a599, a600)
                            };
                            let a602 = 0u32;
                            if equal(a601, a602) {
                                let a605 = {
                                    let a603 = v0;
                                    let a604 = 1u32;
                                    fixed_u32::bit_and(a603, a604)
                                };
                                let a606 = 0u32;
                                if equal(a605, a606) {
                                    Bytes::lit(&[0x24])
                                } else {
                                    Bytes::lit(&[0x25])
                                }
                            } else {
                                let a609 = {
                                    let a607 = v0;
                                    let a608 = 1u32;
                                    fixed_u32::bit_and(a607, a608)
                                };
                                let a610 = 0u32;
                                if equal(a609, a610) {
                                    Bytes::lit(&[0x26])
                                } else {
                                    Bytes::lit(&[0x27])
                                }
                            }
                        }
                    } else {
                        let a613 = {
                            let a611 = v0;
                            let a612 = 4u32;
                            fixed_u32::bit_and(a611, a612)
                        };
                        let a614 = 0u32;
                        if equal(a613, a614) {
                            let a617 = {
                                let a615 = v0;
                                let a616 = 2u32;
                                fixed_u32::bit_and(a615, a616)
                            };
                            let a618 = 0u32;
                            if equal(a617, a618) {
                                let a621 = {
                                    let a619 = v0;
                                    let a620 = 1u32;
                                    fixed_u32::bit_and(a619, a620)
                                };
                                let a622 = 0u32;
                                if equal(a621, a622) {
                                    Bytes::lit(&[0x28])
                                } else {
                                    Bytes::lit(&[0x29])
                                }
                            } else {
                                let a625 = {
                                    let a623 = v0;
                                    let a624 = 1u32;
                                    fixed_u32::bit_and(a623, a624)
                                };
                                let a626 = 0u32;
                                if equal(a625, a626) {
                                    Bytes::lit(&[0x2a])
                                } else {
                                    Bytes::lit(&[0x2b])
                                }
                            }
                        } else {
                            let a629 = {
                                let a627 = v0;
                                let a628 = 2u32;
                                fixed_u32::bit_and(a627, a628)
                            };
                            let a630 = 0u32;
                            if equal(a629, a630) {
                                let a633 = {
                                    let a631 = v0;
                                    let a632 = 1u32;
                                    fixed_u32::bit_and(a631, a632)
                                };
                                let a634 = 0u32;
                                if equal(a633, a634) {
                                    Bytes::lit(&[0x2c])
                                } else {
                                    Bytes::lit(&[0x2d])
                                }
                            } else {
                                let a637 = {
                                    let a635 = v0;
                                    let a636 = 1u32;
                                    fixed_u32::bit_and(a635, a636)
                                };
                                let a638 = 0u32;
                                if equal(a637, a638) {
                                    Bytes::lit(&[0x2e])
                                } else {
                                    Bytes::lit(&[0x2f])
                                }
                            }
                        }
                    }
                } else {
                    let a641 = {
                        let a639 = v0;
                        let a640 = 8u32;
                        fixed_u32::bit_and(a639, a640)
                    };
                    let a642 = 0u32;
                    if equal(a641, a642) {
                        let a645 = {
                            let a643 = v0;
                            let a644 = 4u32;
                            fixed_u32::bit_and(a643, a644)
                        };
                        let a646 = 0u32;
                        if equal(a645, a646) {
                            let a649 = {
                                let a647 = v0;
                                let a648 = 2u32;
                                fixed_u32::bit_and(a647, a648)
                            };
                            let a650 = 0u32;
                            if equal(a649, a650) {
                                let a653 = {
                                    let a651 = v0;
                                    let a652 = 1u32;
                                    fixed_u32::bit_and(a651, a652)
                                };
                                let a654 = 0u32;
                                if equal(a653, a654) {
                                    Bytes::lit(&[0x30])
                                } else {
                                    Bytes::lit(&[0x31])
                                }
                            } else {
                                let a657 = {
                                    let a655 = v0;
                                    let a656 = 1u32;
                                    fixed_u32::bit_and(a655, a656)
                                };
                                let a658 = 0u32;
                                if equal(a657, a658) {
                                    Bytes::lit(&[0x32])
                                } else {
                                    Bytes::lit(&[0x33])
                                }
                            }
                        } else {
                            let a661 = {
                                let a659 = v0;
                                let a660 = 2u32;
                                fixed_u32::bit_and(a659, a660)
                            };
                            let a662 = 0u32;
                            if equal(a661, a662) {
                                let a665 = {
                                    let a663 = v0;
                                    let a664 = 1u32;
                                    fixed_u32::bit_and(a663, a664)
                                };
                                let a666 = 0u32;
                                if equal(a665, a666) {
                                    Bytes::lit(&[0x34])
                                } else {
                                    Bytes::lit(&[0x35])
                                }
                            } else {
                                let a669 = {
                                    let a667 = v0;
                                    let a668 = 1u32;
                                    fixed_u32::bit_and(a667, a668)
                                };
                                let a670 = 0u32;
                                if equal(a669, a670) {
                                    Bytes::lit(&[0x36])
                                } else {
                                    Bytes::lit(&[0x37])
                                }
                            }
                        }
                    } else {
                        let a673 = {
                            let a671 = v0;
                            let a672 = 4u32;
                            fixed_u32::bit_and(a671, a672)
                        };
                        let a674 = 0u32;
                        if equal(a673, a674) {
                            let a677 = {
                                let a675 = v0;
                                let a676 = 2u32;
                                fixed_u32::bit_and(a675, a676)
                            };
                            let a678 = 0u32;
                            if equal(a677, a678) {
                                let a681 = {
                                    let a679 = v0;
                                    let a680 = 1u32;
                                    fixed_u32::bit_and(a679, a680)
                                };
                                let a682 = 0u32;
                                if equal(a681, a682) {
                                    Bytes::lit(&[0x38])
                                } else {
                                    Bytes::lit(&[0x39])
                                }
                            } else {
                                let a685 = {
                                    let a683 = v0;
                                    let a684 = 1u32;
                                    fixed_u32::bit_and(a683, a684)
                                };
                                let a686 = 0u32;
                                if equal(a685, a686) {
                                    Bytes::lit(&[0x3a])
                                } else {
                                    Bytes::lit(&[0x3b])
                                }
                            }
                        } else {
                            let a689 = {
                                let a687 = v0;
                                let a688 = 2u32;
                                fixed_u32::bit_and(a687, a688)
                            };
                            let a690 = 0u32;
                            if equal(a689, a690) {
                                let a693 = {
                                    let a691 = v0;
                                    let a692 = 1u32;
                                    fixed_u32::bit_and(a691, a692)
                                };
                                let a694 = 0u32;
                                if equal(a693, a694) {
                                    Bytes::lit(&[0x3c])
                                } else {
                                    Bytes::lit(&[0x3d])
                                }
                            } else {
                                let a697 = {
                                    let a695 = v0;
                                    let a696 = 1u32;
                                    fixed_u32::bit_and(a695, a696)
                                };
                                let a698 = 0u32;
                                if equal(a697, a698) {
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
            let a701 = {
                let a699 = v0;
                let a700 = 32u32;
                fixed_u32::bit_and(a699, a700)
            };
            let a702 = 0u32;
            if equal(a701, a702) {
                let a705 = {
                    let a703 = v0;
                    let a704 = 16u32;
                    fixed_u32::bit_and(a703, a704)
                };
                let a706 = 0u32;
                if equal(a705, a706) {
                    let a709 = {
                        let a707 = v0;
                        let a708 = 8u32;
                        fixed_u32::bit_and(a707, a708)
                    };
                    let a710 = 0u32;
                    if equal(a709, a710) {
                        let a713 = {
                            let a711 = v0;
                            let a712 = 4u32;
                            fixed_u32::bit_and(a711, a712)
                        };
                        let a714 = 0u32;
                        if equal(a713, a714) {
                            let a717 = {
                                let a715 = v0;
                                let a716 = 2u32;
                                fixed_u32::bit_and(a715, a716)
                            };
                            let a718 = 0u32;
                            if equal(a717, a718) {
                                let a721 = {
                                    let a719 = v0;
                                    let a720 = 1u32;
                                    fixed_u32::bit_and(a719, a720)
                                };
                                let a722 = 0u32;
                                if equal(a721, a722) {
                                    Bytes::lit(&[0x40])
                                } else {
                                    Bytes::lit(&[0x41])
                                }
                            } else {
                                let a725 = {
                                    let a723 = v0;
                                    let a724 = 1u32;
                                    fixed_u32::bit_and(a723, a724)
                                };
                                let a726 = 0u32;
                                if equal(a725, a726) {
                                    Bytes::lit(&[0x42])
                                } else {
                                    Bytes::lit(&[0x43])
                                }
                            }
                        } else {
                            let a729 = {
                                let a727 = v0;
                                let a728 = 2u32;
                                fixed_u32::bit_and(a727, a728)
                            };
                            let a730 = 0u32;
                            if equal(a729, a730) {
                                let a733 = {
                                    let a731 = v0;
                                    let a732 = 1u32;
                                    fixed_u32::bit_and(a731, a732)
                                };
                                let a734 = 0u32;
                                if equal(a733, a734) {
                                    Bytes::lit(&[0x44])
                                } else {
                                    Bytes::lit(&[0x45])
                                }
                            } else {
                                let a737 = {
                                    let a735 = v0;
                                    let a736 = 1u32;
                                    fixed_u32::bit_and(a735, a736)
                                };
                                let a738 = 0u32;
                                if equal(a737, a738) {
                                    Bytes::lit(&[0x46])
                                } else {
                                    Bytes::lit(&[0x47])
                                }
                            }
                        }
                    } else {
                        let a741 = {
                            let a739 = v0;
                            let a740 = 4u32;
                            fixed_u32::bit_and(a739, a740)
                        };
                        let a742 = 0u32;
                        if equal(a741, a742) {
                            let a745 = {
                                let a743 = v0;
                                let a744 = 2u32;
                                fixed_u32::bit_and(a743, a744)
                            };
                            let a746 = 0u32;
                            if equal(a745, a746) {
                                let a749 = {
                                    let a747 = v0;
                                    let a748 = 1u32;
                                    fixed_u32::bit_and(a747, a748)
                                };
                                let a750 = 0u32;
                                if equal(a749, a750) {
                                    Bytes::lit(&[0x48])
                                } else {
                                    Bytes::lit(&[0x49])
                                }
                            } else {
                                let a753 = {
                                    let a751 = v0;
                                    let a752 = 1u32;
                                    fixed_u32::bit_and(a751, a752)
                                };
                                let a754 = 0u32;
                                if equal(a753, a754) {
                                    Bytes::lit(&[0x4a])
                                } else {
                                    Bytes::lit(&[0x4b])
                                }
                            }
                        } else {
                            let a757 = {
                                let a755 = v0;
                                let a756 = 2u32;
                                fixed_u32::bit_and(a755, a756)
                            };
                            let a758 = 0u32;
                            if equal(a757, a758) {
                                let a761 = {
                                    let a759 = v0;
                                    let a760 = 1u32;
                                    fixed_u32::bit_and(a759, a760)
                                };
                                let a762 = 0u32;
                                if equal(a761, a762) {
                                    Bytes::lit(&[0x4c])
                                } else {
                                    Bytes::lit(&[0x4d])
                                }
                            } else {
                                let a765 = {
                                    let a763 = v0;
                                    let a764 = 1u32;
                                    fixed_u32::bit_and(a763, a764)
                                };
                                let a766 = 0u32;
                                if equal(a765, a766) {
                                    Bytes::lit(&[0x4e])
                                } else {
                                    Bytes::lit(&[0x4f])
                                }
                            }
                        }
                    }
                } else {
                    let a769 = {
                        let a767 = v0;
                        let a768 = 8u32;
                        fixed_u32::bit_and(a767, a768)
                    };
                    let a770 = 0u32;
                    if equal(a769, a770) {
                        let a773 = {
                            let a771 = v0;
                            let a772 = 4u32;
                            fixed_u32::bit_and(a771, a772)
                        };
                        let a774 = 0u32;
                        if equal(a773, a774) {
                            let a777 = {
                                let a775 = v0;
                                let a776 = 2u32;
                                fixed_u32::bit_and(a775, a776)
                            };
                            let a778 = 0u32;
                            if equal(a777, a778) {
                                let a781 = {
                                    let a779 = v0;
                                    let a780 = 1u32;
                                    fixed_u32::bit_and(a779, a780)
                                };
                                let a782 = 0u32;
                                if equal(a781, a782) {
                                    Bytes::lit(&[0x50])
                                } else {
                                    Bytes::lit(&[0x51])
                                }
                            } else {
                                let a785 = {
                                    let a783 = v0;
                                    let a784 = 1u32;
                                    fixed_u32::bit_and(a783, a784)
                                };
                                let a786 = 0u32;
                                if equal(a785, a786) {
                                    Bytes::lit(&[0x52])
                                } else {
                                    Bytes::lit(&[0x53])
                                }
                            }
                        } else {
                            let a789 = {
                                let a787 = v0;
                                let a788 = 2u32;
                                fixed_u32::bit_and(a787, a788)
                            };
                            let a790 = 0u32;
                            if equal(a789, a790) {
                                let a793 = {
                                    let a791 = v0;
                                    let a792 = 1u32;
                                    fixed_u32::bit_and(a791, a792)
                                };
                                let a794 = 0u32;
                                if equal(a793, a794) {
                                    Bytes::lit(&[0x54])
                                } else {
                                    Bytes::lit(&[0x55])
                                }
                            } else {
                                let a797 = {
                                    let a795 = v0;
                                    let a796 = 1u32;
                                    fixed_u32::bit_and(a795, a796)
                                };
                                let a798 = 0u32;
                                if equal(a797, a798) {
                                    Bytes::lit(&[0x56])
                                } else {
                                    Bytes::lit(&[0x57])
                                }
                            }
                        }
                    } else {
                        let a801 = {
                            let a799 = v0;
                            let a800 = 4u32;
                            fixed_u32::bit_and(a799, a800)
                        };
                        let a802 = 0u32;
                        if equal(a801, a802) {
                            let a805 = {
                                let a803 = v0;
                                let a804 = 2u32;
                                fixed_u32::bit_and(a803, a804)
                            };
                            let a806 = 0u32;
                            if equal(a805, a806) {
                                let a809 = {
                                    let a807 = v0;
                                    let a808 = 1u32;
                                    fixed_u32::bit_and(a807, a808)
                                };
                                let a810 = 0u32;
                                if equal(a809, a810) {
                                    Bytes::lit(&[0x58])
                                } else {
                                    Bytes::lit(&[0x59])
                                }
                            } else {
                                let a813 = {
                                    let a811 = v0;
                                    let a812 = 1u32;
                                    fixed_u32::bit_and(a811, a812)
                                };
                                let a814 = 0u32;
                                if equal(a813, a814) {
                                    Bytes::lit(&[0x5a])
                                } else {
                                    Bytes::lit(&[0x5b])
                                }
                            }
                        } else {
                            let a817 = {
                                let a815 = v0;
                                let a816 = 2u32;
                                fixed_u32::bit_and(a815, a816)
                            };
                            let a818 = 0u32;
                            if equal(a817, a818) {
                                let a821 = {
                                    let a819 = v0;
                                    let a820 = 1u32;
                                    fixed_u32::bit_and(a819, a820)
                                };
                                let a822 = 0u32;
                                if equal(a821, a822) {
                                    Bytes::lit(&[0x5c])
                                } else {
                                    Bytes::lit(&[0x5d])
                                }
                            } else {
                                let a825 = {
                                    let a823 = v0;
                                    let a824 = 1u32;
                                    fixed_u32::bit_and(a823, a824)
                                };
                                let a826 = 0u32;
                                if equal(a825, a826) {
                                    Bytes::lit(&[0x5e])
                                } else {
                                    Bytes::lit(&[0x5f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a829 = {
                    let a827 = v0;
                    let a828 = 16u32;
                    fixed_u32::bit_and(a827, a828)
                };
                let a830 = 0u32;
                if equal(a829, a830) {
                    let a833 = {
                        let a831 = v0;
                        let a832 = 8u32;
                        fixed_u32::bit_and(a831, a832)
                    };
                    let a834 = 0u32;
                    if equal(a833, a834) {
                        let a837 = {
                            let a835 = v0;
                            let a836 = 4u32;
                            fixed_u32::bit_and(a835, a836)
                        };
                        let a838 = 0u32;
                        if equal(a837, a838) {
                            let a841 = {
                                let a839 = v0;
                                let a840 = 2u32;
                                fixed_u32::bit_and(a839, a840)
                            };
                            let a842 = 0u32;
                            if equal(a841, a842) {
                                let a845 = {
                                    let a843 = v0;
                                    let a844 = 1u32;
                                    fixed_u32::bit_and(a843, a844)
                                };
                                let a846 = 0u32;
                                if equal(a845, a846) {
                                    Bytes::lit(&[0x60])
                                } else {
                                    Bytes::lit(&[0x61])
                                }
                            } else {
                                let a849 = {
                                    let a847 = v0;
                                    let a848 = 1u32;
                                    fixed_u32::bit_and(a847, a848)
                                };
                                let a850 = 0u32;
                                if equal(a849, a850) {
                                    Bytes::lit(&[0x62])
                                } else {
                                    Bytes::lit(&[0x63])
                                }
                            }
                        } else {
                            let a853 = {
                                let a851 = v0;
                                let a852 = 2u32;
                                fixed_u32::bit_and(a851, a852)
                            };
                            let a854 = 0u32;
                            if equal(a853, a854) {
                                let a857 = {
                                    let a855 = v0;
                                    let a856 = 1u32;
                                    fixed_u32::bit_and(a855, a856)
                                };
                                let a858 = 0u32;
                                if equal(a857, a858) {
                                    Bytes::lit(&[0x64])
                                } else {
                                    Bytes::lit(&[0x65])
                                }
                            } else {
                                let a861 = {
                                    let a859 = v0;
                                    let a860 = 1u32;
                                    fixed_u32::bit_and(a859, a860)
                                };
                                let a862 = 0u32;
                                if equal(a861, a862) {
                                    Bytes::lit(&[0x66])
                                } else {
                                    Bytes::lit(&[0x67])
                                }
                            }
                        }
                    } else {
                        let a865 = {
                            let a863 = v0;
                            let a864 = 4u32;
                            fixed_u32::bit_and(a863, a864)
                        };
                        let a866 = 0u32;
                        if equal(a865, a866) {
                            let a869 = {
                                let a867 = v0;
                                let a868 = 2u32;
                                fixed_u32::bit_and(a867, a868)
                            };
                            let a870 = 0u32;
                            if equal(a869, a870) {
                                let a873 = {
                                    let a871 = v0;
                                    let a872 = 1u32;
                                    fixed_u32::bit_and(a871, a872)
                                };
                                let a874 = 0u32;
                                if equal(a873, a874) {
                                    Bytes::lit(&[0x68])
                                } else {
                                    Bytes::lit(&[0x69])
                                }
                            } else {
                                let a877 = {
                                    let a875 = v0;
                                    let a876 = 1u32;
                                    fixed_u32::bit_and(a875, a876)
                                };
                                let a878 = 0u32;
                                if equal(a877, a878) {
                                    Bytes::lit(&[0x6a])
                                } else {
                                    Bytes::lit(&[0x6b])
                                }
                            }
                        } else {
                            let a881 = {
                                let a879 = v0;
                                let a880 = 2u32;
                                fixed_u32::bit_and(a879, a880)
                            };
                            let a882 = 0u32;
                            if equal(a881, a882) {
                                let a885 = {
                                    let a883 = v0;
                                    let a884 = 1u32;
                                    fixed_u32::bit_and(a883, a884)
                                };
                                let a886 = 0u32;
                                if equal(a885, a886) {
                                    Bytes::lit(&[0x6c])
                                } else {
                                    Bytes::lit(&[0x6d])
                                }
                            } else {
                                let a889 = {
                                    let a887 = v0;
                                    let a888 = 1u32;
                                    fixed_u32::bit_and(a887, a888)
                                };
                                let a890 = 0u32;
                                if equal(a889, a890) {
                                    Bytes::lit(&[0x6e])
                                } else {
                                    Bytes::lit(&[0x6f])
                                }
                            }
                        }
                    }
                } else {
                    let a893 = {
                        let a891 = v0;
                        let a892 = 8u32;
                        fixed_u32::bit_and(a891, a892)
                    };
                    let a894 = 0u32;
                    if equal(a893, a894) {
                        let a897 = {
                            let a895 = v0;
                            let a896 = 4u32;
                            fixed_u32::bit_and(a895, a896)
                        };
                        let a898 = 0u32;
                        if equal(a897, a898) {
                            let a901 = {
                                let a899 = v0;
                                let a900 = 2u32;
                                fixed_u32::bit_and(a899, a900)
                            };
                            let a902 = 0u32;
                            if equal(a901, a902) {
                                let a905 = {
                                    let a903 = v0;
                                    let a904 = 1u32;
                                    fixed_u32::bit_and(a903, a904)
                                };
                                let a906 = 0u32;
                                if equal(a905, a906) {
                                    Bytes::lit(&[0x70])
                                } else {
                                    Bytes::lit(&[0x71])
                                }
                            } else {
                                let a909 = {
                                    let a907 = v0;
                                    let a908 = 1u32;
                                    fixed_u32::bit_and(a907, a908)
                                };
                                let a910 = 0u32;
                                if equal(a909, a910) {
                                    Bytes::lit(&[0x72])
                                } else {
                                    Bytes::lit(&[0x73])
                                }
                            }
                        } else {
                            let a913 = {
                                let a911 = v0;
                                let a912 = 2u32;
                                fixed_u32::bit_and(a911, a912)
                            };
                            let a914 = 0u32;
                            if equal(a913, a914) {
                                let a917 = {
                                    let a915 = v0;
                                    let a916 = 1u32;
                                    fixed_u32::bit_and(a915, a916)
                                };
                                let a918 = 0u32;
                                if equal(a917, a918) {
                                    Bytes::lit(&[0x74])
                                } else {
                                    Bytes::lit(&[0x75])
                                }
                            } else {
                                let a921 = {
                                    let a919 = v0;
                                    let a920 = 1u32;
                                    fixed_u32::bit_and(a919, a920)
                                };
                                let a922 = 0u32;
                                if equal(a921, a922) {
                                    Bytes::lit(&[0x76])
                                } else {
                                    Bytes::lit(&[0x77])
                                }
                            }
                        }
                    } else {
                        let a925 = {
                            let a923 = v0;
                            let a924 = 4u32;
                            fixed_u32::bit_and(a923, a924)
                        };
                        let a926 = 0u32;
                        if equal(a925, a926) {
                            let a929 = {
                                let a927 = v0;
                                let a928 = 2u32;
                                fixed_u32::bit_and(a927, a928)
                            };
                            let a930 = 0u32;
                            if equal(a929, a930) {
                                let a933 = {
                                    let a931 = v0;
                                    let a932 = 1u32;
                                    fixed_u32::bit_and(a931, a932)
                                };
                                let a934 = 0u32;
                                if equal(a933, a934) {
                                    Bytes::lit(&[0x78])
                                } else {
                                    Bytes::lit(&[0x79])
                                }
                            } else {
                                let a937 = {
                                    let a935 = v0;
                                    let a936 = 1u32;
                                    fixed_u32::bit_and(a935, a936)
                                };
                                let a938 = 0u32;
                                if equal(a937, a938) {
                                    Bytes::lit(&[0x7a])
                                } else {
                                    Bytes::lit(&[0x7b])
                                }
                            }
                        } else {
                            let a941 = {
                                let a939 = v0;
                                let a940 = 2u32;
                                fixed_u32::bit_and(a939, a940)
                            };
                            let a942 = 0u32;
                            if equal(a941, a942) {
                                let a945 = {
                                    let a943 = v0;
                                    let a944 = 1u32;
                                    fixed_u32::bit_and(a943, a944)
                                };
                                let a946 = 0u32;
                                if equal(a945, a946) {
                                    Bytes::lit(&[0x7c])
                                } else {
                                    Bytes::lit(&[0x7d])
                                }
                            } else {
                                let a949 = {
                                    let a947 = v0;
                                    let a948 = 1u32;
                                    fixed_u32::bit_and(a947, a948)
                                };
                                let a950 = 0u32;
                                if equal(a949, a950) {
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
        let a953 = {
            let a951 = v0;
            let a952 = 64u32;
            fixed_u32::bit_and(a951, a952)
        };
        let a954 = 0u32;
        if equal(a953, a954) {
            let a957 = {
                let a955 = v0;
                let a956 = 32u32;
                fixed_u32::bit_and(a955, a956)
            };
            let a958 = 0u32;
            if equal(a957, a958) {
                let a961 = {
                    let a959 = v0;
                    let a960 = 16u32;
                    fixed_u32::bit_and(a959, a960)
                };
                let a962 = 0u32;
                if equal(a961, a962) {
                    let a965 = {
                        let a963 = v0;
                        let a964 = 8u32;
                        fixed_u32::bit_and(a963, a964)
                    };
                    let a966 = 0u32;
                    if equal(a965, a966) {
                        let a969 = {
                            let a967 = v0;
                            let a968 = 4u32;
                            fixed_u32::bit_and(a967, a968)
                        };
                        let a970 = 0u32;
                        if equal(a969, a970) {
                            let a973 = {
                                let a971 = v0;
                                let a972 = 2u32;
                                fixed_u32::bit_and(a971, a972)
                            };
                            let a974 = 0u32;
                            if equal(a973, a974) {
                                let a977 = {
                                    let a975 = v0;
                                    let a976 = 1u32;
                                    fixed_u32::bit_and(a975, a976)
                                };
                                let a978 = 0u32;
                                if equal(a977, a978) {
                                    Bytes::lit(&[0x80])
                                } else {
                                    Bytes::lit(&[0x81])
                                }
                            } else {
                                let a981 = {
                                    let a979 = v0;
                                    let a980 = 1u32;
                                    fixed_u32::bit_and(a979, a980)
                                };
                                let a982 = 0u32;
                                if equal(a981, a982) {
                                    Bytes::lit(&[0x82])
                                } else {
                                    Bytes::lit(&[0x83])
                                }
                            }
                        } else {
                            let a985 = {
                                let a983 = v0;
                                let a984 = 2u32;
                                fixed_u32::bit_and(a983, a984)
                            };
                            let a986 = 0u32;
                            if equal(a985, a986) {
                                let a989 = {
                                    let a987 = v0;
                                    let a988 = 1u32;
                                    fixed_u32::bit_and(a987, a988)
                                };
                                let a990 = 0u32;
                                if equal(a989, a990) {
                                    Bytes::lit(&[0x84])
                                } else {
                                    Bytes::lit(&[0x85])
                                }
                            } else {
                                let a993 = {
                                    let a991 = v0;
                                    let a992 = 1u32;
                                    fixed_u32::bit_and(a991, a992)
                                };
                                let a994 = 0u32;
                                if equal(a993, a994) {
                                    Bytes::lit(&[0x86])
                                } else {
                                    Bytes::lit(&[0x87])
                                }
                            }
                        }
                    } else {
                        let a997 = {
                            let a995 = v0;
                            let a996 = 4u32;
                            fixed_u32::bit_and(a995, a996)
                        };
                        let a998 = 0u32;
                        if equal(a997, a998) {
                            let a1001 = {
                                let a999 = v0;
                                let a1000 = 2u32;
                                fixed_u32::bit_and(a999, a1000)
                            };
                            let a1002 = 0u32;
                            if equal(a1001, a1002) {
                                let a1005 = {
                                    let a1003 = v0;
                                    let a1004 = 1u32;
                                    fixed_u32::bit_and(a1003, a1004)
                                };
                                let a1006 = 0u32;
                                if equal(a1005, a1006) {
                                    Bytes::lit(&[0x88])
                                } else {
                                    Bytes::lit(&[0x89])
                                }
                            } else {
                                let a1009 = {
                                    let a1007 = v0;
                                    let a1008 = 1u32;
                                    fixed_u32::bit_and(a1007, a1008)
                                };
                                let a1010 = 0u32;
                                if equal(a1009, a1010) {
                                    Bytes::lit(&[0x8a])
                                } else {
                                    Bytes::lit(&[0x8b])
                                }
                            }
                        } else {
                            let a1013 = {
                                let a1011 = v0;
                                let a1012 = 2u32;
                                fixed_u32::bit_and(a1011, a1012)
                            };
                            let a1014 = 0u32;
                            if equal(a1013, a1014) {
                                let a1017 = {
                                    let a1015 = v0;
                                    let a1016 = 1u32;
                                    fixed_u32::bit_and(a1015, a1016)
                                };
                                let a1018 = 0u32;
                                if equal(a1017, a1018) {
                                    Bytes::lit(&[0x8c])
                                } else {
                                    Bytes::lit(&[0x8d])
                                }
                            } else {
                                let a1021 = {
                                    let a1019 = v0;
                                    let a1020 = 1u32;
                                    fixed_u32::bit_and(a1019, a1020)
                                };
                                let a1022 = 0u32;
                                if equal(a1021, a1022) {
                                    Bytes::lit(&[0x8e])
                                } else {
                                    Bytes::lit(&[0x8f])
                                }
                            }
                        }
                    }
                } else {
                    let a1025 = {
                        let a1023 = v0;
                        let a1024 = 8u32;
                        fixed_u32::bit_and(a1023, a1024)
                    };
                    let a1026 = 0u32;
                    if equal(a1025, a1026) {
                        let a1029 = {
                            let a1027 = v0;
                            let a1028 = 4u32;
                            fixed_u32::bit_and(a1027, a1028)
                        };
                        let a1030 = 0u32;
                        if equal(a1029, a1030) {
                            let a1033 = {
                                let a1031 = v0;
                                let a1032 = 2u32;
                                fixed_u32::bit_and(a1031, a1032)
                            };
                            let a1034 = 0u32;
                            if equal(a1033, a1034) {
                                let a1037 = {
                                    let a1035 = v0;
                                    let a1036 = 1u32;
                                    fixed_u32::bit_and(a1035, a1036)
                                };
                                let a1038 = 0u32;
                                if equal(a1037, a1038) {
                                    Bytes::lit(&[0x90])
                                } else {
                                    Bytes::lit(&[0x91])
                                }
                            } else {
                                let a1041 = {
                                    let a1039 = v0;
                                    let a1040 = 1u32;
                                    fixed_u32::bit_and(a1039, a1040)
                                };
                                let a1042 = 0u32;
                                if equal(a1041, a1042) {
                                    Bytes::lit(&[0x92])
                                } else {
                                    Bytes::lit(&[0x93])
                                }
                            }
                        } else {
                            let a1045 = {
                                let a1043 = v0;
                                let a1044 = 2u32;
                                fixed_u32::bit_and(a1043, a1044)
                            };
                            let a1046 = 0u32;
                            if equal(a1045, a1046) {
                                let a1049 = {
                                    let a1047 = v0;
                                    let a1048 = 1u32;
                                    fixed_u32::bit_and(a1047, a1048)
                                };
                                let a1050 = 0u32;
                                if equal(a1049, a1050) {
                                    Bytes::lit(&[0x94])
                                } else {
                                    Bytes::lit(&[0x95])
                                }
                            } else {
                                let a1053 = {
                                    let a1051 = v0;
                                    let a1052 = 1u32;
                                    fixed_u32::bit_and(a1051, a1052)
                                };
                                let a1054 = 0u32;
                                if equal(a1053, a1054) {
                                    Bytes::lit(&[0x96])
                                } else {
                                    Bytes::lit(&[0x97])
                                }
                            }
                        }
                    } else {
                        let a1057 = {
                            let a1055 = v0;
                            let a1056 = 4u32;
                            fixed_u32::bit_and(a1055, a1056)
                        };
                        let a1058 = 0u32;
                        if equal(a1057, a1058) {
                            let a1061 = {
                                let a1059 = v0;
                                let a1060 = 2u32;
                                fixed_u32::bit_and(a1059, a1060)
                            };
                            let a1062 = 0u32;
                            if equal(a1061, a1062) {
                                let a1065 = {
                                    let a1063 = v0;
                                    let a1064 = 1u32;
                                    fixed_u32::bit_and(a1063, a1064)
                                };
                                let a1066 = 0u32;
                                if equal(a1065, a1066) {
                                    Bytes::lit(&[0x98])
                                } else {
                                    Bytes::lit(&[0x99])
                                }
                            } else {
                                let a1069 = {
                                    let a1067 = v0;
                                    let a1068 = 1u32;
                                    fixed_u32::bit_and(a1067, a1068)
                                };
                                let a1070 = 0u32;
                                if equal(a1069, a1070) {
                                    Bytes::lit(&[0x9a])
                                } else {
                                    Bytes::lit(&[0x9b])
                                }
                            }
                        } else {
                            let a1073 = {
                                let a1071 = v0;
                                let a1072 = 2u32;
                                fixed_u32::bit_and(a1071, a1072)
                            };
                            let a1074 = 0u32;
                            if equal(a1073, a1074) {
                                let a1077 = {
                                    let a1075 = v0;
                                    let a1076 = 1u32;
                                    fixed_u32::bit_and(a1075, a1076)
                                };
                                let a1078 = 0u32;
                                if equal(a1077, a1078) {
                                    Bytes::lit(&[0x9c])
                                } else {
                                    Bytes::lit(&[0x9d])
                                }
                            } else {
                                let a1081 = {
                                    let a1079 = v0;
                                    let a1080 = 1u32;
                                    fixed_u32::bit_and(a1079, a1080)
                                };
                                let a1082 = 0u32;
                                if equal(a1081, a1082) {
                                    Bytes::lit(&[0x9e])
                                } else {
                                    Bytes::lit(&[0x9f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a1085 = {
                    let a1083 = v0;
                    let a1084 = 16u32;
                    fixed_u32::bit_and(a1083, a1084)
                };
                let a1086 = 0u32;
                if equal(a1085, a1086) {
                    let a1089 = {
                        let a1087 = v0;
                        let a1088 = 8u32;
                        fixed_u32::bit_and(a1087, a1088)
                    };
                    let a1090 = 0u32;
                    if equal(a1089, a1090) {
                        let a1093 = {
                            let a1091 = v0;
                            let a1092 = 4u32;
                            fixed_u32::bit_and(a1091, a1092)
                        };
                        let a1094 = 0u32;
                        if equal(a1093, a1094) {
                            let a1097 = {
                                let a1095 = v0;
                                let a1096 = 2u32;
                                fixed_u32::bit_and(a1095, a1096)
                            };
                            let a1098 = 0u32;
                            if equal(a1097, a1098) {
                                let a1101 = {
                                    let a1099 = v0;
                                    let a1100 = 1u32;
                                    fixed_u32::bit_and(a1099, a1100)
                                };
                                let a1102 = 0u32;
                                if equal(a1101, a1102) {
                                    Bytes::lit(&[0xa0])
                                } else {
                                    Bytes::lit(&[0xa1])
                                }
                            } else {
                                let a1105 = {
                                    let a1103 = v0;
                                    let a1104 = 1u32;
                                    fixed_u32::bit_and(a1103, a1104)
                                };
                                let a1106 = 0u32;
                                if equal(a1105, a1106) {
                                    Bytes::lit(&[0xa2])
                                } else {
                                    Bytes::lit(&[0xa3])
                                }
                            }
                        } else {
                            let a1109 = {
                                let a1107 = v0;
                                let a1108 = 2u32;
                                fixed_u32::bit_and(a1107, a1108)
                            };
                            let a1110 = 0u32;
                            if equal(a1109, a1110) {
                                let a1113 = {
                                    let a1111 = v0;
                                    let a1112 = 1u32;
                                    fixed_u32::bit_and(a1111, a1112)
                                };
                                let a1114 = 0u32;
                                if equal(a1113, a1114) {
                                    Bytes::lit(&[0xa4])
                                } else {
                                    Bytes::lit(&[0xa5])
                                }
                            } else {
                                let a1117 = {
                                    let a1115 = v0;
                                    let a1116 = 1u32;
                                    fixed_u32::bit_and(a1115, a1116)
                                };
                                let a1118 = 0u32;
                                if equal(a1117, a1118) {
                                    Bytes::lit(&[0xa6])
                                } else {
                                    Bytes::lit(&[0xa7])
                                }
                            }
                        }
                    } else {
                        let a1121 = {
                            let a1119 = v0;
                            let a1120 = 4u32;
                            fixed_u32::bit_and(a1119, a1120)
                        };
                        let a1122 = 0u32;
                        if equal(a1121, a1122) {
                            let a1125 = {
                                let a1123 = v0;
                                let a1124 = 2u32;
                                fixed_u32::bit_and(a1123, a1124)
                            };
                            let a1126 = 0u32;
                            if equal(a1125, a1126) {
                                let a1129 = {
                                    let a1127 = v0;
                                    let a1128 = 1u32;
                                    fixed_u32::bit_and(a1127, a1128)
                                };
                                let a1130 = 0u32;
                                if equal(a1129, a1130) {
                                    Bytes::lit(&[0xa8])
                                } else {
                                    Bytes::lit(&[0xa9])
                                }
                            } else {
                                let a1133 = {
                                    let a1131 = v0;
                                    let a1132 = 1u32;
                                    fixed_u32::bit_and(a1131, a1132)
                                };
                                let a1134 = 0u32;
                                if equal(a1133, a1134) {
                                    Bytes::lit(&[0xaa])
                                } else {
                                    Bytes::lit(&[0xab])
                                }
                            }
                        } else {
                            let a1137 = {
                                let a1135 = v0;
                                let a1136 = 2u32;
                                fixed_u32::bit_and(a1135, a1136)
                            };
                            let a1138 = 0u32;
                            if equal(a1137, a1138) {
                                let a1141 = {
                                    let a1139 = v0;
                                    let a1140 = 1u32;
                                    fixed_u32::bit_and(a1139, a1140)
                                };
                                let a1142 = 0u32;
                                if equal(a1141, a1142) {
                                    Bytes::lit(&[0xac])
                                } else {
                                    Bytes::lit(&[0xad])
                                }
                            } else {
                                let a1145 = {
                                    let a1143 = v0;
                                    let a1144 = 1u32;
                                    fixed_u32::bit_and(a1143, a1144)
                                };
                                let a1146 = 0u32;
                                if equal(a1145, a1146) {
                                    Bytes::lit(&[0xae])
                                } else {
                                    Bytes::lit(&[0xaf])
                                }
                            }
                        }
                    }
                } else {
                    let a1149 = {
                        let a1147 = v0;
                        let a1148 = 8u32;
                        fixed_u32::bit_and(a1147, a1148)
                    };
                    let a1150 = 0u32;
                    if equal(a1149, a1150) {
                        let a1153 = {
                            let a1151 = v0;
                            let a1152 = 4u32;
                            fixed_u32::bit_and(a1151, a1152)
                        };
                        let a1154 = 0u32;
                        if equal(a1153, a1154) {
                            let a1157 = {
                                let a1155 = v0;
                                let a1156 = 2u32;
                                fixed_u32::bit_and(a1155, a1156)
                            };
                            let a1158 = 0u32;
                            if equal(a1157, a1158) {
                                let a1161 = {
                                    let a1159 = v0;
                                    let a1160 = 1u32;
                                    fixed_u32::bit_and(a1159, a1160)
                                };
                                let a1162 = 0u32;
                                if equal(a1161, a1162) {
                                    Bytes::lit(&[0xb0])
                                } else {
                                    Bytes::lit(&[0xb1])
                                }
                            } else {
                                let a1165 = {
                                    let a1163 = v0;
                                    let a1164 = 1u32;
                                    fixed_u32::bit_and(a1163, a1164)
                                };
                                let a1166 = 0u32;
                                if equal(a1165, a1166) {
                                    Bytes::lit(&[0xb2])
                                } else {
                                    Bytes::lit(&[0xb3])
                                }
                            }
                        } else {
                            let a1169 = {
                                let a1167 = v0;
                                let a1168 = 2u32;
                                fixed_u32::bit_and(a1167, a1168)
                            };
                            let a1170 = 0u32;
                            if equal(a1169, a1170) {
                                let a1173 = {
                                    let a1171 = v0;
                                    let a1172 = 1u32;
                                    fixed_u32::bit_and(a1171, a1172)
                                };
                                let a1174 = 0u32;
                                if equal(a1173, a1174) {
                                    Bytes::lit(&[0xb4])
                                } else {
                                    Bytes::lit(&[0xb5])
                                }
                            } else {
                                let a1177 = {
                                    let a1175 = v0;
                                    let a1176 = 1u32;
                                    fixed_u32::bit_and(a1175, a1176)
                                };
                                let a1178 = 0u32;
                                if equal(a1177, a1178) {
                                    Bytes::lit(&[0xb6])
                                } else {
                                    Bytes::lit(&[0xb7])
                                }
                            }
                        }
                    } else {
                        let a1181 = {
                            let a1179 = v0;
                            let a1180 = 4u32;
                            fixed_u32::bit_and(a1179, a1180)
                        };
                        let a1182 = 0u32;
                        if equal(a1181, a1182) {
                            let a1185 = {
                                let a1183 = v0;
                                let a1184 = 2u32;
                                fixed_u32::bit_and(a1183, a1184)
                            };
                            let a1186 = 0u32;
                            if equal(a1185, a1186) {
                                let a1189 = {
                                    let a1187 = v0;
                                    let a1188 = 1u32;
                                    fixed_u32::bit_and(a1187, a1188)
                                };
                                let a1190 = 0u32;
                                if equal(a1189, a1190) {
                                    Bytes::lit(&[0xb8])
                                } else {
                                    Bytes::lit(&[0xb9])
                                }
                            } else {
                                let a1193 = {
                                    let a1191 = v0;
                                    let a1192 = 1u32;
                                    fixed_u32::bit_and(a1191, a1192)
                                };
                                let a1194 = 0u32;
                                if equal(a1193, a1194) {
                                    Bytes::lit(&[0xba])
                                } else {
                                    Bytes::lit(&[0xbb])
                                }
                            }
                        } else {
                            let a1197 = {
                                let a1195 = v0;
                                let a1196 = 2u32;
                                fixed_u32::bit_and(a1195, a1196)
                            };
                            let a1198 = 0u32;
                            if equal(a1197, a1198) {
                                let a1201 = {
                                    let a1199 = v0;
                                    let a1200 = 1u32;
                                    fixed_u32::bit_and(a1199, a1200)
                                };
                                let a1202 = 0u32;
                                if equal(a1201, a1202) {
                                    Bytes::lit(&[0xbc])
                                } else {
                                    Bytes::lit(&[0xbd])
                                }
                            } else {
                                let a1205 = {
                                    let a1203 = v0;
                                    let a1204 = 1u32;
                                    fixed_u32::bit_and(a1203, a1204)
                                };
                                let a1206 = 0u32;
                                if equal(a1205, a1206) {
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
            let a1209 = {
                let a1207 = v0;
                let a1208 = 32u32;
                fixed_u32::bit_and(a1207, a1208)
            };
            let a1210 = 0u32;
            if equal(a1209, a1210) {
                let a1213 = {
                    let a1211 = v0;
                    let a1212 = 16u32;
                    fixed_u32::bit_and(a1211, a1212)
                };
                let a1214 = 0u32;
                if equal(a1213, a1214) {
                    let a1217 = {
                        let a1215 = v0;
                        let a1216 = 8u32;
                        fixed_u32::bit_and(a1215, a1216)
                    };
                    let a1218 = 0u32;
                    if equal(a1217, a1218) {
                        let a1221 = {
                            let a1219 = v0;
                            let a1220 = 4u32;
                            fixed_u32::bit_and(a1219, a1220)
                        };
                        let a1222 = 0u32;
                        if equal(a1221, a1222) {
                            let a1225 = {
                                let a1223 = v0;
                                let a1224 = 2u32;
                                fixed_u32::bit_and(a1223, a1224)
                            };
                            let a1226 = 0u32;
                            if equal(a1225, a1226) {
                                let a1229 = {
                                    let a1227 = v0;
                                    let a1228 = 1u32;
                                    fixed_u32::bit_and(a1227, a1228)
                                };
                                let a1230 = 0u32;
                                if equal(a1229, a1230) {
                                    Bytes::lit(&[0xc0])
                                } else {
                                    Bytes::lit(&[0xc1])
                                }
                            } else {
                                let a1233 = {
                                    let a1231 = v0;
                                    let a1232 = 1u32;
                                    fixed_u32::bit_and(a1231, a1232)
                                };
                                let a1234 = 0u32;
                                if equal(a1233, a1234) {
                                    Bytes::lit(&[0xc2])
                                } else {
                                    Bytes::lit(&[0xc3])
                                }
                            }
                        } else {
                            let a1237 = {
                                let a1235 = v0;
                                let a1236 = 2u32;
                                fixed_u32::bit_and(a1235, a1236)
                            };
                            let a1238 = 0u32;
                            if equal(a1237, a1238) {
                                let a1241 = {
                                    let a1239 = v0;
                                    let a1240 = 1u32;
                                    fixed_u32::bit_and(a1239, a1240)
                                };
                                let a1242 = 0u32;
                                if equal(a1241, a1242) {
                                    Bytes::lit(&[0xc4])
                                } else {
                                    Bytes::lit(&[0xc5])
                                }
                            } else {
                                let a1245 = {
                                    let a1243 = v0;
                                    let a1244 = 1u32;
                                    fixed_u32::bit_and(a1243, a1244)
                                };
                                let a1246 = 0u32;
                                if equal(a1245, a1246) {
                                    Bytes::lit(&[0xc6])
                                } else {
                                    Bytes::lit(&[0xc7])
                                }
                            }
                        }
                    } else {
                        let a1249 = {
                            let a1247 = v0;
                            let a1248 = 4u32;
                            fixed_u32::bit_and(a1247, a1248)
                        };
                        let a1250 = 0u32;
                        if equal(a1249, a1250) {
                            let a1253 = {
                                let a1251 = v0;
                                let a1252 = 2u32;
                                fixed_u32::bit_and(a1251, a1252)
                            };
                            let a1254 = 0u32;
                            if equal(a1253, a1254) {
                                let a1257 = {
                                    let a1255 = v0;
                                    let a1256 = 1u32;
                                    fixed_u32::bit_and(a1255, a1256)
                                };
                                let a1258 = 0u32;
                                if equal(a1257, a1258) {
                                    Bytes::lit(&[0xc8])
                                } else {
                                    Bytes::lit(&[0xc9])
                                }
                            } else {
                                let a1261 = {
                                    let a1259 = v0;
                                    let a1260 = 1u32;
                                    fixed_u32::bit_and(a1259, a1260)
                                };
                                let a1262 = 0u32;
                                if equal(a1261, a1262) {
                                    Bytes::lit(&[0xca])
                                } else {
                                    Bytes::lit(&[0xcb])
                                }
                            }
                        } else {
                            let a1265 = {
                                let a1263 = v0;
                                let a1264 = 2u32;
                                fixed_u32::bit_and(a1263, a1264)
                            };
                            let a1266 = 0u32;
                            if equal(a1265, a1266) {
                                let a1269 = {
                                    let a1267 = v0;
                                    let a1268 = 1u32;
                                    fixed_u32::bit_and(a1267, a1268)
                                };
                                let a1270 = 0u32;
                                if equal(a1269, a1270) {
                                    Bytes::lit(&[0xcc])
                                } else {
                                    Bytes::lit(&[0xcd])
                                }
                            } else {
                                let a1273 = {
                                    let a1271 = v0;
                                    let a1272 = 1u32;
                                    fixed_u32::bit_and(a1271, a1272)
                                };
                                let a1274 = 0u32;
                                if equal(a1273, a1274) {
                                    Bytes::lit(&[0xce])
                                } else {
                                    Bytes::lit(&[0xcf])
                                }
                            }
                        }
                    }
                } else {
                    let a1277 = {
                        let a1275 = v0;
                        let a1276 = 8u32;
                        fixed_u32::bit_and(a1275, a1276)
                    };
                    let a1278 = 0u32;
                    if equal(a1277, a1278) {
                        let a1281 = {
                            let a1279 = v0;
                            let a1280 = 4u32;
                            fixed_u32::bit_and(a1279, a1280)
                        };
                        let a1282 = 0u32;
                        if equal(a1281, a1282) {
                            let a1285 = {
                                let a1283 = v0;
                                let a1284 = 2u32;
                                fixed_u32::bit_and(a1283, a1284)
                            };
                            let a1286 = 0u32;
                            if equal(a1285, a1286) {
                                let a1289 = {
                                    let a1287 = v0;
                                    let a1288 = 1u32;
                                    fixed_u32::bit_and(a1287, a1288)
                                };
                                let a1290 = 0u32;
                                if equal(a1289, a1290) {
                                    Bytes::lit(&[0xd0])
                                } else {
                                    Bytes::lit(&[0xd1])
                                }
                            } else {
                                let a1293 = {
                                    let a1291 = v0;
                                    let a1292 = 1u32;
                                    fixed_u32::bit_and(a1291, a1292)
                                };
                                let a1294 = 0u32;
                                if equal(a1293, a1294) {
                                    Bytes::lit(&[0xd2])
                                } else {
                                    Bytes::lit(&[0xd3])
                                }
                            }
                        } else {
                            let a1297 = {
                                let a1295 = v0;
                                let a1296 = 2u32;
                                fixed_u32::bit_and(a1295, a1296)
                            };
                            let a1298 = 0u32;
                            if equal(a1297, a1298) {
                                let a1301 = {
                                    let a1299 = v0;
                                    let a1300 = 1u32;
                                    fixed_u32::bit_and(a1299, a1300)
                                };
                                let a1302 = 0u32;
                                if equal(a1301, a1302) {
                                    Bytes::lit(&[0xd4])
                                } else {
                                    Bytes::lit(&[0xd5])
                                }
                            } else {
                                let a1305 = {
                                    let a1303 = v0;
                                    let a1304 = 1u32;
                                    fixed_u32::bit_and(a1303, a1304)
                                };
                                let a1306 = 0u32;
                                if equal(a1305, a1306) {
                                    Bytes::lit(&[0xd6])
                                } else {
                                    Bytes::lit(&[0xd7])
                                }
                            }
                        }
                    } else {
                        let a1309 = {
                            let a1307 = v0;
                            let a1308 = 4u32;
                            fixed_u32::bit_and(a1307, a1308)
                        };
                        let a1310 = 0u32;
                        if equal(a1309, a1310) {
                            let a1313 = {
                                let a1311 = v0;
                                let a1312 = 2u32;
                                fixed_u32::bit_and(a1311, a1312)
                            };
                            let a1314 = 0u32;
                            if equal(a1313, a1314) {
                                let a1317 = {
                                    let a1315 = v0;
                                    let a1316 = 1u32;
                                    fixed_u32::bit_and(a1315, a1316)
                                };
                                let a1318 = 0u32;
                                if equal(a1317, a1318) {
                                    Bytes::lit(&[0xd8])
                                } else {
                                    Bytes::lit(&[0xd9])
                                }
                            } else {
                                let a1321 = {
                                    let a1319 = v0;
                                    let a1320 = 1u32;
                                    fixed_u32::bit_and(a1319, a1320)
                                };
                                let a1322 = 0u32;
                                if equal(a1321, a1322) {
                                    Bytes::lit(&[0xda])
                                } else {
                                    Bytes::lit(&[0xdb])
                                }
                            }
                        } else {
                            let a1325 = {
                                let a1323 = v0;
                                let a1324 = 2u32;
                                fixed_u32::bit_and(a1323, a1324)
                            };
                            let a1326 = 0u32;
                            if equal(a1325, a1326) {
                                let a1329 = {
                                    let a1327 = v0;
                                    let a1328 = 1u32;
                                    fixed_u32::bit_and(a1327, a1328)
                                };
                                let a1330 = 0u32;
                                if equal(a1329, a1330) {
                                    Bytes::lit(&[0xdc])
                                } else {
                                    Bytes::lit(&[0xdd])
                                }
                            } else {
                                let a1333 = {
                                    let a1331 = v0;
                                    let a1332 = 1u32;
                                    fixed_u32::bit_and(a1331, a1332)
                                };
                                let a1334 = 0u32;
                                if equal(a1333, a1334) {
                                    Bytes::lit(&[0xde])
                                } else {
                                    Bytes::lit(&[0xdf])
                                }
                            }
                        }
                    }
                }
            } else {
                let a1337 = {
                    let a1335 = v0;
                    let a1336 = 16u32;
                    fixed_u32::bit_and(a1335, a1336)
                };
                let a1338 = 0u32;
                if equal(a1337, a1338) {
                    let a1341 = {
                        let a1339 = v0;
                        let a1340 = 8u32;
                        fixed_u32::bit_and(a1339, a1340)
                    };
                    let a1342 = 0u32;
                    if equal(a1341, a1342) {
                        let a1345 = {
                            let a1343 = v0;
                            let a1344 = 4u32;
                            fixed_u32::bit_and(a1343, a1344)
                        };
                        let a1346 = 0u32;
                        if equal(a1345, a1346) {
                            let a1349 = {
                                let a1347 = v0;
                                let a1348 = 2u32;
                                fixed_u32::bit_and(a1347, a1348)
                            };
                            let a1350 = 0u32;
                            if equal(a1349, a1350) {
                                let a1353 = {
                                    let a1351 = v0;
                                    let a1352 = 1u32;
                                    fixed_u32::bit_and(a1351, a1352)
                                };
                                let a1354 = 0u32;
                                if equal(a1353, a1354) {
                                    Bytes::lit(&[0xe0])
                                } else {
                                    Bytes::lit(&[0xe1])
                                }
                            } else {
                                let a1357 = {
                                    let a1355 = v0;
                                    let a1356 = 1u32;
                                    fixed_u32::bit_and(a1355, a1356)
                                };
                                let a1358 = 0u32;
                                if equal(a1357, a1358) {
                                    Bytes::lit(&[0xe2])
                                } else {
                                    Bytes::lit(&[0xe3])
                                }
                            }
                        } else {
                            let a1361 = {
                                let a1359 = v0;
                                let a1360 = 2u32;
                                fixed_u32::bit_and(a1359, a1360)
                            };
                            let a1362 = 0u32;
                            if equal(a1361, a1362) {
                                let a1365 = {
                                    let a1363 = v0;
                                    let a1364 = 1u32;
                                    fixed_u32::bit_and(a1363, a1364)
                                };
                                let a1366 = 0u32;
                                if equal(a1365, a1366) {
                                    Bytes::lit(&[0xe4])
                                } else {
                                    Bytes::lit(&[0xe5])
                                }
                            } else {
                                let a1369 = {
                                    let a1367 = v0;
                                    let a1368 = 1u32;
                                    fixed_u32::bit_and(a1367, a1368)
                                };
                                let a1370 = 0u32;
                                if equal(a1369, a1370) {
                                    Bytes::lit(&[0xe6])
                                } else {
                                    Bytes::lit(&[0xe7])
                                }
                            }
                        }
                    } else {
                        let a1373 = {
                            let a1371 = v0;
                            let a1372 = 4u32;
                            fixed_u32::bit_and(a1371, a1372)
                        };
                        let a1374 = 0u32;
                        if equal(a1373, a1374) {
                            let a1377 = {
                                let a1375 = v0;
                                let a1376 = 2u32;
                                fixed_u32::bit_and(a1375, a1376)
                            };
                            let a1378 = 0u32;
                            if equal(a1377, a1378) {
                                let a1381 = {
                                    let a1379 = v0;
                                    let a1380 = 1u32;
                                    fixed_u32::bit_and(a1379, a1380)
                                };
                                let a1382 = 0u32;
                                if equal(a1381, a1382) {
                                    Bytes::lit(&[0xe8])
                                } else {
                                    Bytes::lit(&[0xe9])
                                }
                            } else {
                                let a1385 = {
                                    let a1383 = v0;
                                    let a1384 = 1u32;
                                    fixed_u32::bit_and(a1383, a1384)
                                };
                                let a1386 = 0u32;
                                if equal(a1385, a1386) {
                                    Bytes::lit(&[0xea])
                                } else {
                                    Bytes::lit(&[0xeb])
                                }
                            }
                        } else {
                            let a1389 = {
                                let a1387 = v0;
                                let a1388 = 2u32;
                                fixed_u32::bit_and(a1387, a1388)
                            };
                            let a1390 = 0u32;
                            if equal(a1389, a1390) {
                                let a1393 = {
                                    let a1391 = v0;
                                    let a1392 = 1u32;
                                    fixed_u32::bit_and(a1391, a1392)
                                };
                                let a1394 = 0u32;
                                if equal(a1393, a1394) {
                                    Bytes::lit(&[0xec])
                                } else {
                                    Bytes::lit(&[0xed])
                                }
                            } else {
                                let a1397 = {
                                    let a1395 = v0;
                                    let a1396 = 1u32;
                                    fixed_u32::bit_and(a1395, a1396)
                                };
                                let a1398 = 0u32;
                                if equal(a1397, a1398) {
                                    Bytes::lit(&[0xee])
                                } else {
                                    Bytes::lit(&[0xef])
                                }
                            }
                        }
                    }
                } else {
                    let a1401 = {
                        let a1399 = v0;
                        let a1400 = 8u32;
                        fixed_u32::bit_and(a1399, a1400)
                    };
                    let a1402 = 0u32;
                    if equal(a1401, a1402) {
                        let a1405 = {
                            let a1403 = v0;
                            let a1404 = 4u32;
                            fixed_u32::bit_and(a1403, a1404)
                        };
                        let a1406 = 0u32;
                        if equal(a1405, a1406) {
                            let a1409 = {
                                let a1407 = v0;
                                let a1408 = 2u32;
                                fixed_u32::bit_and(a1407, a1408)
                            };
                            let a1410 = 0u32;
                            if equal(a1409, a1410) {
                                let a1413 = {
                                    let a1411 = v0;
                                    let a1412 = 1u32;
                                    fixed_u32::bit_and(a1411, a1412)
                                };
                                let a1414 = 0u32;
                                if equal(a1413, a1414) {
                                    Bytes::lit(&[0xf0])
                                } else {
                                    Bytes::lit(&[0xf1])
                                }
                            } else {
                                let a1417 = {
                                    let a1415 = v0;
                                    let a1416 = 1u32;
                                    fixed_u32::bit_and(a1415, a1416)
                                };
                                let a1418 = 0u32;
                                if equal(a1417, a1418) {
                                    Bytes::lit(&[0xf2])
                                } else {
                                    Bytes::lit(&[0xf3])
                                }
                            }
                        } else {
                            let a1421 = {
                                let a1419 = v0;
                                let a1420 = 2u32;
                                fixed_u32::bit_and(a1419, a1420)
                            };
                            let a1422 = 0u32;
                            if equal(a1421, a1422) {
                                let a1425 = {
                                    let a1423 = v0;
                                    let a1424 = 1u32;
                                    fixed_u32::bit_and(a1423, a1424)
                                };
                                let a1426 = 0u32;
                                if equal(a1425, a1426) {
                                    Bytes::lit(&[0xf4])
                                } else {
                                    Bytes::lit(&[0xf5])
                                }
                            } else {
                                let a1429 = {
                                    let a1427 = v0;
                                    let a1428 = 1u32;
                                    fixed_u32::bit_and(a1427, a1428)
                                };
                                let a1430 = 0u32;
                                if equal(a1429, a1430) {
                                    Bytes::lit(&[0xf6])
                                } else {
                                    Bytes::lit(&[0xf7])
                                }
                            }
                        }
                    } else {
                        let a1433 = {
                            let a1431 = v0;
                            let a1432 = 4u32;
                            fixed_u32::bit_and(a1431, a1432)
                        };
                        let a1434 = 0u32;
                        if equal(a1433, a1434) {
                            let a1437 = {
                                let a1435 = v0;
                                let a1436 = 2u32;
                                fixed_u32::bit_and(a1435, a1436)
                            };
                            let a1438 = 0u32;
                            if equal(a1437, a1438) {
                                let a1441 = {
                                    let a1439 = v0;
                                    let a1440 = 1u32;
                                    fixed_u32::bit_and(a1439, a1440)
                                };
                                let a1442 = 0u32;
                                if equal(a1441, a1442) {
                                    Bytes::lit(&[0xf8])
                                } else {
                                    Bytes::lit(&[0xf9])
                                }
                            } else {
                                let a1445 = {
                                    let a1443 = v0;
                                    let a1444 = 1u32;
                                    fixed_u32::bit_and(a1443, a1444)
                                };
                                let a1446 = 0u32;
                                if equal(a1445, a1446) {
                                    Bytes::lit(&[0xfa])
                                } else {
                                    Bytes::lit(&[0xfb])
                                }
                            }
                        } else {
                            let a1449 = {
                                let a1447 = v0;
                                let a1448 = 2u32;
                                fixed_u32::bit_and(a1447, a1448)
                            };
                            let a1450 = 0u32;
                            if equal(a1449, a1450) {
                                let a1453 = {
                                    let a1451 = v0;
                                    let a1452 = 1u32;
                                    fixed_u32::bit_and(a1451, a1452)
                                };
                                let a1454 = 0u32;
                                if equal(a1453, a1454) {
                                    Bytes::lit(&[0xfc])
                                } else {
                                    Bytes::lit(&[0xfd])
                                }
                            } else {
                                let a1457 = {
                                    let a1455 = v0;
                                    let a1456 = 1u32;
                                    fixed_u32::bit_and(a1455, a1456)
                                };
                                let a1458 = 0u32;
                                if equal(a1457, a1458) {
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

pub fn f51(v0: u32, v1: u32, v2: u32, v3: u32) -> u32 {
    let a1463 = v0;
    let a1464 = {
        let a1461 = f52(v1, 8u32);
        let a1462 = {
            let a1459 = f52(v2, 16u32);
            let a1460 = f52(v3, 24u32);
            fixed_u32::bit_or(a1459, a1460)
        };
        fixed_u32::bit_or(a1461, a1462)
    };
    fixed_u32::bit_or(a1463, a1464)
}

pub fn f52(v0: u32, v1: u32) -> u32 {
    let m1467 = {
        let a1465 = v0;
        let a1466 = v1;
        fixed_u32::shift_left(a1465, a1466)
    };
    match m1467 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f53(v0: Adt3, v1: Adt3) -> Adt3 {
    f55(f56(f57(f58(f59(f60(f61(f62(v0, v1.clone()), v1.clone()), v1.clone()), v1.clone()), v1.clone()), v1.clone()), v1.clone()), v1)
}

pub fn f54(v0: Adt3) -> Adt3 {
    Adt3::C0(match v0.clone() {
        Adt3::C0(_, _, h1468, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1468
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h1469, _, _, _, _, _, _, _, _, _) => {
            h1469
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h1470, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1470
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h1471, _, _, _, _, _) => {
            h1471
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, h1472, _, _, _, _, _, _, _, _) => {
            h1472
        }
    }, match v0.clone() {
        Adt3::C0(h1473, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1473
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h1474, _, _, _, _, _, _, _, _, _, _, _) => {
            h1474
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1475, _, _) => {
            h1475
        }
    }, match v0.clone() {
        Adt3::C0(_, h1476, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1476
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h1477, _, _, _, _) => {
            h1477
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1478, _, _, _) => {
            h1478
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h1479, _, _, _, _, _, _, _, _, _, _) => {
            h1479
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h1480, _, _, _, _, _, _) => {
            h1480
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1481, _) => {
            h1481
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1482) => {
            h1482
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, _, h1483, _, _, _, _, _, _, _) => {
            h1483
        }
    })
}

pub fn f55(v0: Adt3, v1: Adt3) -> Adt3 {
    let v2: Adt4 = f63(match v0.clone() {
        Adt3::C0(_, _, _, h1484, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1484
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h1485, _, _, _, _, _, _, _, _, _, _, _) => {
            h1485
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h1486, _, _, _, _, _, _) => {
            h1486
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1487, _) => {
            h1487
        }
    }, match v1.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1488, _) => {
            h1488
        }
    }, match v1 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1489) => {
            h1489
        }
    });
    Adt3::C0(match v0.clone() {
        Adt3::C0(h1490, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1490
        }
    }, match v0.clone() {
        Adt3::C0(_, h1491, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1491
        }
    }, match v0.clone() {
        Adt3::C0(_, _, h1492, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1492
        }
    }, match v2.clone() {
        Adt4::C0(h1493, _, _, _) => {
            h1493
        }
    }, match v2.clone() {
        Adt4::C0(_, h1494, _, _) => {
            h1494
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h1495, _, _, _, _, _, _, _, _, _, _) => {
            h1495
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h1496, _, _, _, _, _, _, _, _, _) => {
            h1496
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, h1497, _, _, _, _, _, _, _, _) => {
            h1497
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, h1498, _, _, _, _, _, _, _) => {
            h1498
        }
    }, match v2.clone() {
        Adt4::C0(_, _, h1499, _) => {
            h1499
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h1500, _, _, _, _, _) => {
            h1500
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h1501, _, _, _, _) => {
            h1501
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1502, _, _, _) => {
            h1502
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1503, _, _) => {
            h1503
        }
    }, match v2 {
        Adt4::C0(_, _, _, h1504) => {
            h1504
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1505) => {
            h1505
        }
    })
}

pub fn f56(v0: Adt3, v1: Adt3) -> Adt3 {
    let v2: Adt4 = f63(match v0.clone() {
        Adt3::C0(_, _, h1506, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1506
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, h1507, _, _, _, _, _, _, _, _) => {
            h1507
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, h1508, _, _, _, _, _, _, _) => {
            h1508
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1509, _, _) => {
            h1509
        }
    }, match v1.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1510, _, _, _) => {
            h1510
        }
    }, match v1 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1511, _, _) => {
            h1511
        }
    });
    Adt3::C0(match v0.clone() {
        Adt3::C0(h1512, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1512
        }
    }, match v0.clone() {
        Adt3::C0(_, h1513, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1513
        }
    }, match v2.clone() {
        Adt4::C0(h1514, _, _, _) => {
            h1514
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h1515, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1515
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h1516, _, _, _, _, _, _, _, _, _, _, _) => {
            h1516
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h1517, _, _, _, _, _, _, _, _, _, _) => {
            h1517
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h1518, _, _, _, _, _, _, _, _, _) => {
            h1518
        }
    }, match v2.clone() {
        Adt4::C0(_, h1519, _, _) => {
            h1519
        }
    }, match v2.clone() {
        Adt4::C0(_, _, h1520, _) => {
            h1520
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h1521, _, _, _, _, _, _) => {
            h1521
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h1522, _, _, _, _, _) => {
            h1522
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h1523, _, _, _, _) => {
            h1523
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1524, _, _, _) => {
            h1524
        }
    }, match v2 {
        Adt4::C0(_, _, _, h1525) => {
            h1525
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1526, _) => {
            h1526
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1527) => {
            h1527
        }
    })
}

pub fn f57(v0: Adt3, v1: Adt3) -> Adt3 {
    let v2: Adt4 = f63(match v0.clone() {
        Adt3::C0(_, h1528, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1528
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h1529, _, _, _, _, _, _, _, _, _) => {
            h1529
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h1530, _, _, _, _) => {
            h1530
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1531, _, _, _) => {
            h1531
        }
    }, match v1.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h1532, _, _, _, _, _) => {
            h1532
        }
    }, match v1 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h1533, _, _, _, _) => {
            h1533
        }
    });
    Adt3::C0(match v0.clone() {
        Adt3::C0(h1534, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1534
        }
    }, match v2.clone() {
        Adt4::C0(h1535, _, _, _) => {
            h1535
        }
    }, match v0.clone() {
        Adt3::C0(_, _, h1536, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1536
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h1537, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1537
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h1538, _, _, _, _, _, _, _, _, _, _, _) => {
            h1538
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h1539, _, _, _, _, _, _, _, _, _, _) => {
            h1539
        }
    }, match v2.clone() {
        Adt4::C0(_, h1540, _, _) => {
            h1540
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, h1541, _, _, _, _, _, _, _, _) => {
            h1541
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, h1542, _, _, _, _, _, _, _) => {
            h1542
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h1543, _, _, _, _, _, _) => {
            h1543
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h1544, _, _, _, _, _) => {
            h1544
        }
    }, match v2.clone() {
        Adt4::C0(_, _, h1545, _) => {
            h1545
        }
    }, match v2 {
        Adt4::C0(_, _, _, h1546) => {
            h1546
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1547, _, _) => {
            h1547
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1548, _) => {
            h1548
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1549) => {
            h1549
        }
    })
}

pub fn f58(v0: Adt3, v1: Adt3) -> Adt3 {
    let v2: Adt4 = f63(match v0.clone() {
        Adt3::C0(h1550, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1550
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h1551, _, _, _, _, _, _, _, _, _, _) => {
            h1551
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h1552, _, _, _, _, _) => {
            h1552
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1553) => {
            h1553
        }
    }, match v1.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, h1554, _, _, _, _, _, _, _) => {
            h1554
        }
    }, match v1 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h1555, _, _, _, _, _, _) => {
            h1555
        }
    });
    Adt3::C0(match v2.clone() {
        Adt4::C0(h1556, _, _, _) => {
            h1556
        }
    }, match v0.clone() {
        Adt3::C0(_, h1557, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1557
        }
    }, match v0.clone() {
        Adt3::C0(_, _, h1558, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1558
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h1559, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1559
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h1560, _, _, _, _, _, _, _, _, _, _, _) => {
            h1560
        }
    }, match v2.clone() {
        Adt4::C0(_, h1561, _, _) => {
            h1561
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h1562, _, _, _, _, _, _, _, _, _) => {
            h1562
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, h1563, _, _, _, _, _, _, _, _) => {
            h1563
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, h1564, _, _, _, _, _, _, _) => {
            h1564
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h1565, _, _, _, _, _, _) => {
            h1565
        }
    }, match v2.clone() {
        Adt4::C0(_, _, h1566, _) => {
            h1566
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h1567, _, _, _, _) => {
            h1567
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1568, _, _, _) => {
            h1568
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1569, _, _) => {
            h1569
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1570, _) => {
            h1570
        }
    }, match v2 {
        Adt4::C0(_, _, _, h1571) => {
            h1571
        }
    })
}

pub fn f59(v0: Adt3, v1: Adt3) -> Adt3 {
    let v2: Adt4 = f63(match v0.clone() {
        Adt3::C0(_, _, _, h1572, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1572
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, h1573, _, _, _, _, _, _, _, _) => {
            h1573
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h1574, _, _, _, _) => {
            h1574
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1575) => {
            h1575
        }
    }, match v1.clone() {
        Adt3::C0(_, _, _, _, _, _, h1576, _, _, _, _, _, _, _, _, _) => {
            h1576
        }
    }, match v1 {
        Adt3::C0(_, _, _, _, _, _, _, h1577, _, _, _, _, _, _, _, _) => {
            h1577
        }
    });
    Adt3::C0(match v0.clone() {
        Adt3::C0(h1578, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1578
        }
    }, match v0.clone() {
        Adt3::C0(_, h1579, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1579
        }
    }, match v0.clone() {
        Adt3::C0(_, _, h1580, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1580
        }
    }, match v2.clone() {
        Adt4::C0(h1581, _, _, _) => {
            h1581
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h1582, _, _, _, _, _, _, _, _, _, _, _) => {
            h1582
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h1583, _, _, _, _, _, _, _, _, _, _) => {
            h1583
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h1584, _, _, _, _, _, _, _, _, _) => {
            h1584
        }
    }, match v2.clone() {
        Adt4::C0(_, h1585, _, _) => {
            h1585
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, h1586, _, _, _, _, _, _, _) => {
            h1586
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h1587, _, _, _, _, _, _) => {
            h1587
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h1588, _, _, _, _, _) => {
            h1588
        }
    }, match v2.clone() {
        Adt4::C0(_, _, h1589, _) => {
            h1589
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1590, _, _, _) => {
            h1590
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1591, _, _) => {
            h1591
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1592, _) => {
            h1592
        }
    }, match v2 {
        Adt4::C0(_, _, _, h1593) => {
            h1593
        }
    })
}

pub fn f60(v0: Adt3, v1: Adt3) -> Adt3 {
    let v2: Adt4 = f63(match v0.clone() {
        Adt3::C0(_, _, h1594, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1594
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h1595, _, _, _, _, _, _, _, _, _) => {
            h1595
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h1596, _, _, _, _, _) => {
            h1596
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1597, _) => {
            h1597
        }
    }, match v1.clone() {
        Adt3::C0(_, _, _, _, h1598, _, _, _, _, _, _, _, _, _, _, _) => {
            h1598
        }
    }, match v1 {
        Adt3::C0(_, _, _, _, _, h1599, _, _, _, _, _, _, _, _, _, _) => {
            h1599
        }
    });
    Adt3::C0(match v0.clone() {
        Adt3::C0(h1600, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1600
        }
    }, match v0.clone() {
        Adt3::C0(_, h1601, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1601
        }
    }, match v2.clone() {
        Adt4::C0(h1602, _, _, _) => {
            h1602
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h1603, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1603
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h1604, _, _, _, _, _, _, _, _, _, _, _) => {
            h1604
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h1605, _, _, _, _, _, _, _, _, _, _) => {
            h1605
        }
    }, match v2.clone() {
        Adt4::C0(_, h1606, _, _) => {
            h1606
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, h1607, _, _, _, _, _, _, _, _) => {
            h1607
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, h1608, _, _, _, _, _, _, _) => {
            h1608
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h1609, _, _, _, _, _, _) => {
            h1609
        }
    }, match v2.clone() {
        Adt4::C0(_, _, h1610, _) => {
            h1610
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h1611, _, _, _, _) => {
            h1611
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1612, _, _, _) => {
            h1612
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1613, _, _) => {
            h1613
        }
    }, match v2 {
        Adt4::C0(_, _, _, h1614) => {
            h1614
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1615) => {
            h1615
        }
    })
}

pub fn f61(v0: Adt3, v1: Adt3) -> Adt3 {
    let v2: Adt4 = f63(match v0.clone() {
        Adt3::C0(_, h1616, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1616
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h1617, _, _, _, _, _, _, _, _, _, _) => {
            h1617
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h1618, _, _, _, _, _, _) => {
            h1618
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1619, _, _) => {
            h1619
        }
    }, match v1.clone() {
        Adt3::C0(_, _, h1620, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1620
        }
    }, match v1 {
        Adt3::C0(_, _, _, h1621, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1621
        }
    });
    Adt3::C0(match v0.clone() {
        Adt3::C0(h1622, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1622
        }
    }, match v2.clone() {
        Adt4::C0(h1623, _, _, _) => {
            h1623
        }
    }, match v0.clone() {
        Adt3::C0(_, _, h1624, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1624
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h1625, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1625
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h1626, _, _, _, _, _, _, _, _, _, _, _) => {
            h1626
        }
    }, match v2.clone() {
        Adt4::C0(_, h1627, _, _) => {
            h1627
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h1628, _, _, _, _, _, _, _, _, _) => {
            h1628
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, h1629, _, _, _, _, _, _, _, _) => {
            h1629
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, h1630, _, _, _, _, _, _, _) => {
            h1630
        }
    }, match v2.clone() {
        Adt4::C0(_, _, h1631, _) => {
            h1631
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h1632, _, _, _, _, _) => {
            h1632
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h1633, _, _, _, _) => {
            h1633
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1634, _, _, _) => {
            h1634
        }
    }, match v2 {
        Adt4::C0(_, _, _, h1635) => {
            h1635
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1636, _) => {
            h1636
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1637) => {
            h1637
        }
    })
}

pub fn f62(v0: Adt3, v1: Adt3) -> Adt3 {
    let v2: Adt4 = f63(match v0.clone() {
        Adt3::C0(h1638, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1638
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h1639, _, _, _, _, _, _, _, _, _, _, _) => {
            h1639
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, h1640, _, _, _, _, _, _, _) => {
            h1640
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1641, _, _, _) => {
            h1641
        }
    }, match v1.clone() {
        Adt3::C0(h1642, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1642
        }
    }, match v1 {
        Adt3::C0(_, h1643, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1643
        }
    });
    Adt3::C0(match v2.clone() {
        Adt4::C0(h1644, _, _, _) => {
            h1644
        }
    }, match v0.clone() {
        Adt3::C0(_, h1645, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1645
        }
    }, match v0.clone() {
        Adt3::C0(_, _, h1646, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1646
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h1647, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1647
        }
    }, match v2.clone() {
        Adt4::C0(_, h1648, _, _) => {
            h1648
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h1649, _, _, _, _, _, _, _, _, _, _) => {
            h1649
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h1650, _, _, _, _, _, _, _, _, _) => {
            h1650
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, h1651, _, _, _, _, _, _, _, _) => {
            h1651
        }
    }, match v2.clone() {
        Adt4::C0(_, _, h1652, _) => {
            h1652
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, h1653, _, _, _, _, _, _) => {
            h1653
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, h1654, _, _, _, _, _) => {
            h1654
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h1655, _, _, _, _) => {
            h1655
        }
    }, match v2 {
        Adt4::C0(_, _, _, h1656) => {
            h1656
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1657, _, _) => {
            h1657
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1658, _) => {
            h1658
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1659) => {
            h1659
        }
    })
}

pub fn f63(v0: u32, v1: u32, v2: u32, v3: u32, v4: u32, v5: u32) -> Adt4 {
    let v6: u32 = f64(f64(v0, v1), v4);
    let v7: u32 = f65(f49(v3, v6), 16u32, 16u32);
    let v8: u32 = f64(v2, v7);
    let v9: u32 = f65(f49(v1, v8), 12u32, 20u32);
    let v10: u32 = f64(f64(v6, v9), v5);
    let v11: u32 = f65(f49(v7, v10), 8u32, 24u32);
    let v12: u32 = f64(v8, v11);
    let v13: u32 = f65(f49(v9, v12), 7u32, 25u32);
    Adt4::C0(v10, v13, v12, v11)
}

pub fn f64(v0: u32, v1: u32) -> u32 {
    let m1662 = {
        let a1660 = f66(v0);
        let a1661 = f66(v1);
        fixed_u64::checked_add(a1660, a1661)
    };
    match m1662 {
        None => {
            0u32
        }
        Some(v2) => {
            f67(v2)
        }
    }
}

pub fn f65(v0: u32, v1: u32, v2: u32) -> u32 {
    let a1663 = f16(v0, v1);
    let a1664 = f52(v0, v2);
    fixed_u32::bit_or(a1663, a1664)
}

pub fn f66(v0: u32) -> u64 {
    let m1666 = {
        let a1665 = v0;
        fixed_u64::convert(i128::from(a1665))
    };
    match m1666 {
        None => {
            0u64
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f67(v0: u64) -> u32 {
    let m1670 = {
        let a1669 = {
            let a1667 = v0;
            let a1668 = 4294967295u64;
            fixed_u64::bit_and(a1667, a1668)
        };
        fixed_u32::convert(i128::from(a1669))
    };
    match m1670 {
        None => {
            0u32
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn run(p0: Bytes) -> R<Bytes> {
    f0(p0)
}
