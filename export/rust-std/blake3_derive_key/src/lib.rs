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
    C0(u64, u64, u64, Adt1, u32, List<Adt2>, List<Adt2>),
}

#[derive(Clone)]
pub enum Adt1 {
    C0(u32, u32, u32, u32, u32, u32, u32, u32),
}

#[derive(Clone)]
pub enum Adt2 {
    C0(Adt3),
    C1(Adt5, u64),
    C2(Adt5, u64),
}

#[derive(Clone)]
pub enum Adt3 {
    C0(u64, u64, u64, u64, u64, Adt1, Adt4, u32, u32, u32, u32, Adt4),
}

#[derive(Clone)]
pub enum Adt4 {
    C0(u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32),
}

#[derive(Clone)]
pub enum Adt5 {
    C0(Adt1, u64, u64),
}

#[derive(Clone)]
pub enum Adt6 {
    C0(Adt1, u32, u64, Bytes, List<Adt5>, List<Adt2>, List<Adt2>),
}

#[derive(Clone)]
pub enum Adt7 {
    C0(u64, u64, u64, u64, u64, Adt1, Adt4, u64, u32, u32),
}

#[derive(Clone)]
pub enum Adt8 {
    C0(Adt5, u64, List<Adt5>, List<Adt2>),
}

#[derive(Clone)]
pub enum Adt9 {
    C0(Adt1, Adt4, u32, u32, u32, u32, List<Adt10>),
}

#[derive(Clone)]
pub enum Adt10 {
    C0(u64, u64, u64, Adt11, u64, u64, u32, u32, u32),
}

#[derive(Clone)]
pub enum Adt11 {
    C0,
    C1,
    C2,
    C3,
    C4,
}

#[derive(Clone)]
pub enum Adt12 {
    C0(Adt4, Adt4, List<Adt10>),
}

#[derive(Clone)]
pub enum Adt13 {
    C0(u64, u64, Adt11, u64, u64),
}

#[derive(Clone)]
pub enum Fn0 {
    F6,
}

impl Fn0 {
    pub fn apply(&self, p0: List<Bytes>, p1: Adt2) -> List<Bytes> {
        match self {
            Fn0::F6 => f6(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn1 {
    F9,
    F60,
}

impl Fn1 {
    pub fn apply(&self, p0: List<Bytes>, p1: Bytes) -> List<Bytes> {
        match self {
            Fn1::F9 => f9(p0, p1),
            Fn1::F60 => f60(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn2 {
    F12(u32, Adt1),
}

impl Fn2 {
    pub fn apply(&self, p0: (Adt7, (List<Adt2>, u64)), p1: Adt5) -> R<(Adt7, (List<Adt2>, u64))> {
        match self {
            Fn2::F12(k0, k1) => f12(*k0, k1.clone(), p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn3 {
    F14((Adt7, (List<Adt2>, u64))),
}

impl Fn3 {
    pub fn apply(&self, p0: (u64, List<Adt2>)) -> R<(u64, List<Adt2>)> {
        match self {
            Fn3::F14(k0) => f14(k0.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn4 {
    F18(u32, Bytes, Adt1, Adt6),
}

impl Fn4 {
    pub fn apply(&self, p0: (u64, (List<Adt5>, List<Adt2>))) -> R<(u64, (List<Adt5>, List<Adt2>))> {
        match self {
            Fn4::F18(k0, k1, k2, k3) => f18(*k0, k1.clone(), k2.clone(), k3.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn5 {
    F25,
}

impl Fn5 {
    pub fn apply(&self, p0: List<Bytes>) -> Option<List<Bytes>> {
        match self {
            Fn5::F25 => f25(p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn6 {
    F28(u32, u64, u64, Adt1, u64, Bytes),
}

impl Fn6 {
    pub fn apply(&self, p0: (u64, (Adt1, List<Adt2>))) -> R<(u64, (Adt1, List<Adt2>))> {
        match self {
            Fn6::F28(k0, k1, k2, k3, k4, k5) => f28(*k0, *k1, *k2, k3.clone(), *k4, k5.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn7 {
    F38,
}

impl Fn7 {
    pub fn apply(&self, p0: List<Adt2>, p1: Adt2) -> List<Adt2> {
        match self {
            Fn7::F38 => f38(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn8 {
    F43(u64),
}

impl Fn8 {
    pub fn apply(&self, p0: Adt4, p1: Adt2) -> Adt4 {
        match self {
            Fn8::F43(k0) => f43(*k0, p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn9 {
    F55(u32, Adt1),
}

impl Fn9 {
    pub fn apply(&self, p0: Adt8) -> R<Option<Adt8>> {
        match self {
            Fn9::F55(k0, k1) => f55(*k0, k1.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn10 {
    F59,
}

impl Fn10 {
    pub fn apply(&self, p0: (Option<Bytes>, List<Bytes>), p1: Bytes) -> (Option<Bytes>, List<Bytes>) {
        match self {
            Fn10::F59 => f59(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn11 {
    F63,
}

impl Fn11 {
    pub fn apply(&self, p0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
        match self {
            Fn11::F63 => f63(p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn12 {
    F71,
}

impl Fn12 {
    pub fn apply(&self, p0: Adt4, p1: Adt10) -> Adt4 {
        match self {
            Fn12::F71 => f71(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn13 {
    F73(Adt1, Adt4),
}

impl Fn13 {
    pub fn apply(&self, p0: (u64, Adt12)) -> R<(u64, Adt12)> {
        match self {
            Fn13::F73(k0, k1) => f73(k0.clone(), k1.clone(), p0),
        }
    }
}

pub fn f0(v0: Bytes, v1: Bytes, v2: u64) -> R<Bytes> {
    Ok(f1(f2(f3(f4(v0)?, v1)?, v2)?))
}

pub fn f1(v0: Adt0) -> Bytes {
    let v1: List<Bytes> = f5(Fn0::F6, List::<Bytes>::nil(), match v0.clone() {
        Adt0::C0(_, _, _, _, _, _, h1) => {
            h1
        }
    });
    let v2: Bytes = f7(f8(Fn1::F9, List::<Bytes>::nil(), v1));
    let m6 = {
        let a3 = v2.clone();
        let a4 = 0u64;
        let a5 = match v0 {
            Adt0::C0(_, _, h2, _, _, _, _) => {
                h2
            }
        };
        slice_bytes(a3, a4, a5)
    };
    match m6 {
        None => {
            v2
        }
        Some(v3) => {
            v3
        }
    }
}

pub fn f2(v0: Adt6, v1: u64) -> R<Adt0> {
    let v2: Adt1 = match v0.clone() {
        Adt6::C0(h7, _, _, _, _, _, _) => {
            h7
        }
    };
    let v3: u32 = match v0.clone() {
        Adt6::C0(_, h8, _, _, _, _, _) => {
            h8
        }
    };
    let v4: Bytes = match v0.clone() {
        Adt6::C0(_, _, _, h9, _, _, _) => {
            h9
        }
    };
    let v5: (Adt7, List<Adt2>) = f10(v2.clone(), v3, v4.clone(), 0u64, {
        let a10 = v4.clone();
        length_bytes(a10)
    }, match v0.clone() {
        Adt6::C0(_, _, h11, _, _, _, _) => {
            h11
        }
    }, match v0.clone() {
        Adt6::C0(_, _, _, _, _, h12, _) => {
            h12
        }
    })?;
    let v6: (Adt7, (List<Adt2>, u64)) = f11(Fn2::F12(v3, v2.clone()), ({
        let (h13, _) = v5;
        h13
    }, ({
        let (_, h14) = v5;
        h14
    }, {
        let a16 = match v0.clone() {
            Adt6::C0(_, _, _, _, h15, _, _) => {
                h15
            }
        };
        length_list(a16)
    })), match v0.clone() {
        Adt6::C0(_, _, _, _, h17, _, _) => {
            h17
        }
    })?;
    let v7: (u64, List<Adt2>) = f13(Fn3::F14(v6.clone()), {
        let a20 = {
            let a18 = v1;
            let a19 = 63u64;
            nat_add(a18, a19)?
        };
        let a21 = 64u64;
        let a22 = 0u64;
        nat_quot(a20, a21, a22)
    }, (0u64, {
        let (h23, _) = {
            let (_, h24) = v6;
            h24
        };
        h23
    }))?;
    Ok(Adt0::C0(f15(v3), {
        let a29 = {
            let a26 = match v0.clone() {
                Adt6::C0(_, _, h25, _, _, _, _) => {
                    h25
                }
            };
            let a27 = 1024u64;
            nat_mul(a26, a27)?
        };
        let a30 = {
            let a28 = v4;
            length_bytes(a28)
        };
        nat_add(a29, a30)?
    }, v1, v2, v3, match v0 {
        Adt6::C0(_, _, _, _, _, _, h31) => {
            h31
        }
    }, f16({
        let (_, h32) = v7;
        h32
    })))
}

pub fn f3(v0: Adt6, v1: Bytes) -> R<Adt6> {
    let v2: Bytes = {
        let a34 = match v0.clone() {
            Adt6::C0(_, _, _, h33, _, _, _) => {
                h33
            }
        };
        let a35 = v1;
        append_bytes(a34, a35)
    };
    let v3: u64 = {
        let a36 = v2.clone();
        length_bytes(a36)
    };
    let v4: u64 = {
        let a37 = v3;
        let a38 = 0u64;
        if nat_eq(a37, a38) {
            0u64
        } else {
            let a41 = {
                let a39 = v3;
                let a40 = 1u64;
                nat_sub(a39, a40)
            };
            let a42 = 1024u64;
            let a43 = 0u64;
            nat_quot(a41, a42, a43)
        }
    };
    let v5: u32 = match v0.clone() {
        Adt6::C0(_, h44, _, _, _, _, _) => {
            h44
        }
    };
    let v6: Adt1 = match v0.clone() {
        Adt6::C0(h45, _, _, _, _, _, _) => {
            h45
        }
    };
    let v7: (u64, (List<Adt5>, List<Adt2>)) = f17(Fn4::F18(v5, v2.clone(), v6.clone(), v0.clone()), v4, (0u64, (match v0.clone() {
        Adt6::C0(_, _, _, _, h46, _, _) => {
            h46
        }
    }, match v0.clone() {
        Adt6::C0(_, _, _, _, _, h47, _) => {
            h47
        }
    })))?;
    let v8: u64 = {
        let a48 = v4;
        let a49 = 1024u64;
        nat_mul(a48, a49)?
    };
    Ok(Adt6::C0(v6, v5, {
        let a51 = match v0.clone() {
            Adt6::C0(_, _, h50, _, _, _, _) => {
                h50
            }
        };
        let a52 = v4;
        nat_add(a51, a52)?
    }, {
        let m58 = {
            let a55 = v2;
            let a56 = v8;
            let a57 = {
                let a53 = v3;
                let a54 = v8;
                nat_sub(a53, a54)
            };
            slice_bytes(a55, a56, a57)
        };
        match m58 {
            None => {
                Bytes::lit(&[])
            }
            Some(v9) => {
                v9
            }
        }
    }, {
        let (h59, _) = {
            let (_, h60) = v7.clone();
            h60
        };
        h59
    }, {
        let (_, h61) = {
            let (_, h62) = v7;
            h62
        };
        h61
    }, match v0 {
        Adt6::C0(_, _, _, _, _, _, h63) => {
            h63
        }
    }))
}

pub fn f4(v0: Bytes) -> R<Adt6> {
    let v1: Adt0 = f19(v0)?;
    Ok(Adt6::C0(f20(f21(v1.clone(), 0u64)), f22(), 0u64, Bytes::lit(&[]), List::<Adt5>::nil(), List::<Adt2>::nil(), match v1 {
        Adt0::C0(_, _, _, _, _, _, h64) => {
            h64
        }
    }))
}

pub fn f5(v0: Fn0, p68: List<Bytes>, p69: List<Adt2>) -> List<Bytes> {
    let m67 = (p68, p69);
    {
        let mut m67 = m67;
        loop {
            let step = {
                let (v1, v2) = m67;
                let m65 = v2;
                match m65.uncons() {
                    None => {
                        Err::<(List<Bytes>, List<Adt2>), List<Bytes>>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(List<Bytes>, List<Adt2>), List<Bytes>>(({
                            let c66: Fn0 = v0.clone();
                            c66.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m67 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f6(v0: List<Bytes>, v1: Adt2) -> List<Bytes> {
    let m70 = v1;
    match m70 {
        Adt2::C0(v2) => {
            let a72 = match v2.clone() {
                Adt3::C0(h71, _, _, _, _, _, _, _, _, _, _, _) => {
                    h71
                }
            };
            let a73 = 2u64;
            if nat_eq(a72, a73) {
                List::cons(f23(match v2 {
                    Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h74) => {
                        h74
                    }
                }), v0)
            } else {
                v0
            }
        }
        Adt2::C1(_, _) => {
            v0
        }
        Adt2::C2(_, _) => {
            v0
        }
    }
}

pub fn f7(v0: List<Bytes>) -> Bytes {
    let m76 = {
        let (h75, _) = f24(Fn5::F25, 64u64, v0);
        h75
    };
    match m76.uncons() {
        None => {
            Bytes::lit(&[])
        }
        Some((v1, _)) => {
            v1
        }
    }
}

pub fn f8(v0: Fn1, p80: List<Bytes>, p81: List<Bytes>) -> List<Bytes> {
    let m79 = (p80, p81);
    {
        let mut m79 = m79;
        loop {
            let step = {
                let (v1, v2) = m79;
                let m77 = v2;
                match m77.uncons() {
                    None => {
                        Err::<(List<Bytes>, List<Bytes>), List<Bytes>>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(List<Bytes>, List<Bytes>), List<Bytes>>(({
                            let c78: Fn1 = v0.clone();
                            c78.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m79 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f9(v0: List<Bytes>, v1: Bytes) -> List<Bytes> {
    List::cons(v1, v0)
}

pub fn f10(v0: Adt1, v1: u32, v2: Bytes, v3: u64, v4: u64, v5: u64, v6: List<Adt2>) -> R<(Adt7, List<Adt2>)> {
    let v7: u64 = {
        let a82 = v3;
        let a83 = v4;
        nat_add(a82, a83)?
    };
    let v8: u64 = f26(v4)?;
    let v9: (u64, (Adt1, List<Adt2>)) = f27(Fn6::F28(v1, v3, v5, v0.clone(), v7, v2.clone()), {
        let a84 = v8;
        let a85 = 64u64;
        let a86 = 0u64;
        nat_quot(a84, a85, a86)
    }, (0u64, (v0.clone(), v6)))?;
    Ok((f29(v0, v1, v2, v7, v3, v5, v8, {
        let (h87, _) = {
            let (_, h88) = v9.clone();
            h88
        };
        h87
    }, {
        let a89 = v4;
        let a90 = v8;
        nat_sub(a89, a90)
    }, {
        let a91 = f30();
        let a92 = f31(v8);
        fixed_u32::bit_or(a91, a92)
    })?, {
        let (_, h93) = {
            let (_, h94) = v9;
            h94
        };
        h93
    }))
}

pub fn f11(v0: Fn2, p98: (Adt7, (List<Adt2>, u64)), p99: List<Adt5>) -> R<(Adt7, (List<Adt2>, u64))> {
    let m97 = (p98, p99);
    {
        let mut m97 = m97;
        loop {
            let step = {
                let (v1, v2) = m97;
                let m95 = v2;
                match m95.uncons() {
                    None => {
                        Err::<((Adt7, (List<Adt2>, u64)), List<Adt5>), R<(Adt7, (List<Adt2>, u64))>>(Ok(v1))
                    }
                    Some((v3, v4)) => {
                        Ok::<((Adt7, (List<Adt2>, u64)), List<Adt5>), R<(Adt7, (List<Adt2>, u64))>>(({
                            let c96: Fn2 = v0.clone();
                            c96.apply(v1, v3)?
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m97 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f12(v0: u32, v1: Adt1, v2: (Adt7, (List<Adt2>, u64)), v3: Adt5) -> R<(Adt7, (List<Adt2>, u64))> {
    let v4: Adt3 = f32({
        let (h100, _) = v2.clone();
        h100
    })?;
    Ok((f33(v3.clone(), Adt5::C0(f20(match v4.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h101) => {
            h101
        }
    }), {
        let m104 = {
            let (h103, _) = v2.clone();
            h103
        };
        match m104 {
            Adt7::C0(_, h102, _, _, _, _, _, _, _, _) => {
                h102
            }
        }
    }, {
        let m107 = {
            let (h106, _) = v2.clone();
            h106
        };
        match m107 {
            Adt7::C0(_, _, h105, _, _, _, _, _, _, _) => {
                h105
            }
        }
    }), v1, v0)?, (List::cons(Adt2::C2(v3, {
        let (_, h108) = {
            let (_, h109) = v2.clone();
            h109
        };
        h108
    }), List::cons(Adt2::C0(v4), {
        let (h110, _) = {
            let (_, h111) = v2.clone();
            h111
        };
        h110
    })), {
        let a114 = {
            let (_, h112) = {
                let (_, h113) = v2;
                h113
            };
            h112
        };
        let a115 = 1u64;
        nat_sub(a114, a115)
    })))
}

pub fn f13(v0: Fn3, p119: u64, p120: (u64, List<Adt2>)) -> R<(u64, List<Adt2>)> {
    let m118 = (p119, p120);
    {
        let mut m118 = m118;
        loop {
            let step = {
                let (v1, v2) = m118;
                let m116 = v1;
                if m116 == 0 {
                    Err::<(u64, (u64, List<Adt2>)), R<(u64, List<Adt2>)>>(Ok(v2))
                } else {
                    let v3 = m116 - 1;
                    Ok::<(u64, (u64, List<Adt2>)), R<(u64, List<Adt2>)>>((v3, {
                        let c117: Fn3 = v0.clone();
                        c117.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m118 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f14(v0: (Adt7, (List<Adt2>, u64)), v1: (u64, List<Adt2>)) -> R<(u64, List<Adt2>)> {
    Ok(({
        let a122 = {
            let (h121, _) = v1.clone();
            h121
        };
        let a123 = 1u64;
        nat_add(a122, a123)?
    }, List::cons(Adt2::C0(f32(f34({
        let (h124, _) = v0;
        h124
    }, {
        let (h125, _) = v1;
        h125
    }))?), {
        let (_, h126) = v1;
        h126
    })))
}

pub fn f15(v0: u32) -> u64 {
    let a127 = v0;
    let a128 = f35();
    if equal(a127, a128) {
        1u64
    } else {
        let a129 = v0;
        let a130 = f22();
        if equal(a129, a130) {
            2u64
        } else {
            let a131 = v0;
            let a132 = f36();
            if equal(a131, a132) {
                3u64
            } else {
                0u64
            }
        }
    }
}

pub fn f16(v0: List<Adt2>) -> List<Adt2> {
    f37(Fn7::F38, List::<Adt2>::nil(), v0)
}

pub fn f17(v0: Fn4, p136: u64, p137: (u64, (List<Adt5>, List<Adt2>))) -> R<(u64, (List<Adt5>, List<Adt2>))> {
    let m135 = (p136, p137);
    {
        let mut m135 = m135;
        loop {
            let step = {
                let (v1, v2) = m135;
                let m133 = v1;
                if m133 == 0 {
                    Err::<(u64, (u64, (List<Adt5>, List<Adt2>))), R<(u64, (List<Adt5>, List<Adt2>))>>(Ok(v2))
                } else {
                    let v3 = m133 - 1;
                    Ok::<(u64, (u64, (List<Adt5>, List<Adt2>))), R<(u64, (List<Adt5>, List<Adt2>))>>((v3, {
                        let c134: Fn4 = v0.clone();
                        c134.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m135 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f18(v0: u32, v1: Bytes, v2: Adt1, v3: Adt6, v4: (u64, (List<Adt5>, List<Adt2>))) -> R<(u64, (List<Adt5>, List<Adt2>))> {
    let v5: u64 = {
        let a140 = match v3 {
            Adt6::C0(_, _, h138, _, _, _, _) => {
                h138
            }
        };
        let a141 = {
            let (h139, _) = v4.clone();
            h139
        };
        nat_add(a140, a141)?
    };
    let v6: (Adt7, List<Adt2>) = f10(v2.clone(), v0, v1, {
        let a143 = {
            let (h142, _) = v4.clone();
            h142
        };
        let a144 = 1024u64;
        nat_mul(a143, a144)?
    }, 1024u64, v5, {
        let (_, h145) = {
            let (_, h146) = v4.clone();
            h146
        };
        h145
    })?;
    let v7: Adt3 = f32({
        let (h147, _) = v6;
        h147
    })?;
    Ok(({
        let a149 = {
            let (h148, _) = v4;
            h148
        };
        let a150 = 1u64;
        nat_add(a149, a150)?
    }, f39(v2, v0, Adt8::C0(Adt5::C0(f20(match v7.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h151) => {
            h151
        }
    }), v5, 1u64), {
        let a152 = v5;
        let a153 = 1u64;
        nat_add(a152, a153)?
    }, {
        let (h154, _) = {
            let (_, h155) = v4;
            h155
        };
        h154
    }, List::cons(Adt2::C0(v7), {
        let (_, h156) = v6;
        h156
    })))?))
}

pub fn f19(v0: Bytes) -> R<Adt0> {
    f2(f3(f40(f41(), f36()), v0)?, 32u64)
}

pub fn f20(v0: Adt4) -> Adt1 {
    Adt1::C0(match v0.clone() {
        Adt4::C0(h157, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h157
        }
    }, match v0.clone() {
        Adt4::C0(_, h158, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h158
        }
    }, match v0.clone() {
        Adt4::C0(_, _, h159, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h159
        }
    }, match v0.clone() {
        Adt4::C0(_, _, _, h160, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h160
        }
    }, match v0.clone() {
        Adt4::C0(_, _, _, _, h161, _, _, _, _, _, _, _, _, _, _, _) => {
            h161
        }
    }, match v0.clone() {
        Adt4::C0(_, _, _, _, _, h162, _, _, _, _, _, _, _, _, _, _) => {
            h162
        }
    }, match v0.clone() {
        Adt4::C0(_, _, _, _, _, _, h163, _, _, _, _, _, _, _, _, _) => {
            h163
        }
    }, match v0 {
        Adt4::C0(_, _, _, _, _, _, _, h164, _, _, _, _, _, _, _, _) => {
            h164
        }
    })
}

pub fn f21(v0: Adt0, v1: u64) -> Adt4 {
    f42(Fn8::F43(v1), Adt4::C0(0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32), match v0 {
        Adt0::C0(_, _, _, _, _, _, h165) => {
            h165
        }
    })
}

pub fn f22() -> u32 {
    64u32
}

pub fn f23(v0: Adt4) -> Bytes {
    let a210 = f44(match v0.clone() {
        Adt4::C0(h166, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h166
        }
    });
    let a211 = {
        let a208 = f44(match v0.clone() {
            Adt4::C0(_, h167, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                h167
            }
        });
        let a209 = {
            let a206 = f44(match v0.clone() {
                Adt4::C0(_, _, h168, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                    h168
                }
            });
            let a207 = {
                let a204 = f44(match v0.clone() {
                    Adt4::C0(_, _, _, h169, _, _, _, _, _, _, _, _, _, _, _, _) => {
                        h169
                    }
                });
                let a205 = {
                    let a202 = f44(match v0.clone() {
                        Adt4::C0(_, _, _, _, h170, _, _, _, _, _, _, _, _, _, _, _) => {
                            h170
                        }
                    });
                    let a203 = {
                        let a200 = f44(match v0.clone() {
                            Adt4::C0(_, _, _, _, _, h171, _, _, _, _, _, _, _, _, _, _) => {
                                h171
                            }
                        });
                        let a201 = {
                            let a198 = f44(match v0.clone() {
                                Adt4::C0(_, _, _, _, _, _, h172, _, _, _, _, _, _, _, _, _) => {
                                    h172
                                }
                            });
                            let a199 = {
                                let a196 = f44(match v0.clone() {
                                    Adt4::C0(_, _, _, _, _, _, _, h173, _, _, _, _, _, _, _, _) => {
                                        h173
                                    }
                                });
                                let a197 = {
                                    let a194 = f44(match v0.clone() {
                                        Adt4::C0(_, _, _, _, _, _, _, _, h174, _, _, _, _, _, _, _) => {
                                            h174
                                        }
                                    });
                                    let a195 = {
                                        let a192 = f44(match v0.clone() {
                                            Adt4::C0(_, _, _, _, _, _, _, _, _, h175, _, _, _, _, _, _) => {
                                                h175
                                            }
                                        });
                                        let a193 = {
                                            let a190 = f44(match v0.clone() {
                                                Adt4::C0(_, _, _, _, _, _, _, _, _, _, h176, _, _, _, _, _) => {
                                                    h176
                                                }
                                            });
                                            let a191 = {
                                                let a188 = f44(match v0.clone() {
                                                    Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h177, _, _, _, _) => {
                                                        h177
                                                    }
                                                });
                                                let a189 = {
                                                    let a186 = f44(match v0.clone() {
                                                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h178, _, _, _) => {
                                                            h178
                                                        }
                                                    });
                                                    let a187 = {
                                                        let a184 = f44(match v0.clone() {
                                                            Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h179, _, _) => {
                                                                h179
                                                            }
                                                        });
                                                        let a185 = {
                                                            let a182 = f44(match v0.clone() {
                                                                Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h180, _) => {
                                                                    h180
                                                                }
                                                            });
                                                            let a183 = f44(match v0 {
                                                                Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h181) => {
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

pub fn f24(v0: Fn5, p216: u64, p217: List<Bytes>) -> (List<Bytes>, bool) {
    let m215 = (p216, p217);
    {
        let mut m215 = m215;
        loop {
            let step = {
                let (v1, v2) = m215;
                let m212 = v1;
                if m212 == 0 {
                    Err::<(u64, List<Bytes>), (List<Bytes>, bool)>((v2, false))
                } else {
                    let v3 = m212 - 1;
                    let m214 = {
                        let c213: Fn5 = v0.clone();
                        c213.apply(v2.clone())
                    };
                    match m214 {
                        None => {
                            Err::<(u64, List<Bytes>), (List<Bytes>, bool)>((v2, true))
                        }
                        Some(v4) => {
                            Ok::<(u64, List<Bytes>), (List<Bytes>, bool)>((v3, v4))
                        }
                    }
                }
            };
            match step {
                Ok(next) => { m215 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f25(v0: List<Bytes>) -> Option<List<Bytes>> {
    let a219 = {
        let a218 = v0.clone();
        length_list(a218)
    };
    let a220 = 1u64;
    if nat_le(a219, a220) {
        None::<List<Bytes>>
    } else {
        Some(f45(v0))
    }
}

pub fn f26(v0: u64) -> R<u64> {
    let a221 = v0;
    let a222 = 0u64;
    if nat_eq(a221, a222) {
        Ok(0u64)
    } else {
        let a228 = {
            let a225 = {
                let a223 = v0;
                let a224 = 1u64;
                nat_sub(a223, a224)
            };
            let a226 = 64u64;
            let a227 = 0u64;
            nat_quot(a225, a226, a227)
        };
        let a229 = 64u64;
        nat_mul(a228, a229)
    }
}

pub fn f27(v0: Fn6, p233: u64, p234: (u64, (Adt1, List<Adt2>))) -> R<(u64, (Adt1, List<Adt2>))> {
    let m232 = (p233, p234);
    {
        let mut m232 = m232;
        loop {
            let step = {
                let (v1, v2) = m232;
                let m230 = v1;
                if m230 == 0 {
                    Err::<(u64, (u64, (Adt1, List<Adt2>))), R<(u64, (Adt1, List<Adt2>))>>(Ok(v2))
                } else {
                    let v3 = m230 - 1;
                    Ok::<(u64, (u64, (Adt1, List<Adt2>))), R<(u64, (Adt1, List<Adt2>))>>((v3, {
                        let c231: Fn6 = v0.clone();
                        c231.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m232 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f28(v0: u32, v1: u64, v2: u64, v3: Adt1, v4: u64, v5: Bytes, v6: (u64, (Adt1, List<Adt2>))) -> R<(u64, (Adt1, List<Adt2>))> {
    let v7: u64 = {
        let (h235, _) = v6.clone();
        h235
    };
    let v8: Adt3 = f32(f29(v3, v0, v5, v4, v1, v2, v7, {
        let (h236, _) = {
            let (_, h237) = v6.clone();
            h237
        };
        h236
    }, 64u64, f31(v7))?)?;
    Ok(({
        let a238 = v7;
        let a239 = 64u64;
        nat_add(a238, a239)?
    }, (f20(match v8.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h240) => {
            h240
        }
    }), List::cons(Adt2::C0(v8), {
        let (_, h241) = {
            let (_, h242) = v6;
            h242
        };
        h241
    }))))
}

pub fn f29(_: Adt1, v1: u32, v2: Bytes, v3: u64, v4: u64, v5: u64, v6: u64, v7: Adt1, v8: u64, v9: u32) -> R<Adt7> {
    Ok(Adt7::C0(0u64, v5, 1u64, {
        let a243 = v6;
        let a244 = 64u64;
        let a245 = 0u64;
        nat_quot(a243, a244, a245)
    }, {
        let a248 = {
            let a246 = v5;
            let a247 = 1024u64;
            nat_mul(a246, a247)?
        };
        let a249 = v6;
        nat_add(a248, a249)?
    }, v7, f46(v2, v3, {
        let a250 = v4;
        let a251 = v6;
        nat_add(a250, a251)?
    })?, v5, f47(v8, 7u64), {
        let a252 = v1;
        let a253 = v9;
        fixed_u32::bit_or(a252, a253)
    }))
}

pub fn f30() -> u32 {
    2u32
}

pub fn f31(v0: u64) -> u32 {
    let a254 = v0;
    let a255 = 0u64;
    if nat_eq(a254, a255) {
        f48()
    } else {
        0u32
    }
}

pub fn f32(v0: Adt7) -> R<Adt3> {
    let v1: u32 = f49(match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, h256, _, _) => {
            h256
        }
    });
    let v2: u32 = f50(match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, h257, _, _) => {
            h257
        }
    });
    Ok(Adt3::C0(match v0.clone() {
        Adt7::C0(h258, _, _, _, _, _, _, _, _, _) => {
            h258
        }
    }, match v0.clone() {
        Adt7::C0(_, h259, _, _, _, _, _, _, _, _) => {
            h259
        }
    }, match v0.clone() {
        Adt7::C0(_, _, h260, _, _, _, _, _, _, _) => {
            h260
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, h261, _, _, _, _, _, _) => {
            h261
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, h262, _, _, _, _, _) => {
            h262
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, h263, _, _, _, _) => {
            h263
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, h264, _, _, _) => {
            h264
        }
    }, v1, v2, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, _, h265, _) => {
            h265
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, _, _, h266) => {
            h266
        }
    }, f51(match v0.clone() {
        Adt7::C0(_, _, _, _, _, h267, _, _, _, _) => {
            h267
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, h268, _, _, _) => {
            h268
        }
    }, v1, v2, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, _, h269, _) => {
            h269
        }
    }, match v0 {
        Adt7::C0(_, _, _, _, _, _, _, _, _, h270) => {
            h270
        }
    })?))
}

pub fn f33(v0: Adt5, v1: Adt5, v2: Adt1, v3: u32) -> R<Adt7> {
    Ok(Adt7::C0(1u64, match v0.clone() {
        Adt5::C0(_, h271, _) => {
            h271
        }
    }, {
        let a274 = match v0.clone() {
            Adt5::C0(_, _, h272) => {
                h272
            }
        };
        let a275 = match v1.clone() {
            Adt5::C0(_, _, h273) => {
                h273
            }
        };
        nat_add(a274, a275)?
    }, 0u64, 0u64, v2, Adt4::C0({
        let m278 = match v0.clone() {
            Adt5::C0(h277, _, _) => {
                h277
            }
        };
        match m278 {
            Adt1::C0(h276, _, _, _, _, _, _, _) => {
                h276
            }
        }
    }, {
        let m281 = match v0.clone() {
            Adt5::C0(h280, _, _) => {
                h280
            }
        };
        match m281 {
            Adt1::C0(_, h279, _, _, _, _, _, _) => {
                h279
            }
        }
    }, {
        let m284 = match v0.clone() {
            Adt5::C0(h283, _, _) => {
                h283
            }
        };
        match m284 {
            Adt1::C0(_, _, h282, _, _, _, _, _) => {
                h282
            }
        }
    }, {
        let m287 = match v0.clone() {
            Adt5::C0(h286, _, _) => {
                h286
            }
        };
        match m287 {
            Adt1::C0(_, _, _, h285, _, _, _, _) => {
                h285
            }
        }
    }, {
        let m290 = match v0.clone() {
            Adt5::C0(h289, _, _) => {
                h289
            }
        };
        match m290 {
            Adt1::C0(_, _, _, _, h288, _, _, _) => {
                h288
            }
        }
    }, {
        let m293 = match v0.clone() {
            Adt5::C0(h292, _, _) => {
                h292
            }
        };
        match m293 {
            Adt1::C0(_, _, _, _, _, h291, _, _) => {
                h291
            }
        }
    }, {
        let m296 = match v0.clone() {
            Adt5::C0(h295, _, _) => {
                h295
            }
        };
        match m296 {
            Adt1::C0(_, _, _, _, _, _, h294, _) => {
                h294
            }
        }
    }, {
        let m299 = match v0 {
            Adt5::C0(h298, _, _) => {
                h298
            }
        };
        match m299 {
            Adt1::C0(_, _, _, _, _, _, _, h297) => {
                h297
            }
        }
    }, {
        let m302 = match v1.clone() {
            Adt5::C0(h301, _, _) => {
                h301
            }
        };
        match m302 {
            Adt1::C0(h300, _, _, _, _, _, _, _) => {
                h300
            }
        }
    }, {
        let m305 = match v1.clone() {
            Adt5::C0(h304, _, _) => {
                h304
            }
        };
        match m305 {
            Adt1::C0(_, h303, _, _, _, _, _, _) => {
                h303
            }
        }
    }, {
        let m308 = match v1.clone() {
            Adt5::C0(h307, _, _) => {
                h307
            }
        };
        match m308 {
            Adt1::C0(_, _, h306, _, _, _, _, _) => {
                h306
            }
        }
    }, {
        let m311 = match v1.clone() {
            Adt5::C0(h310, _, _) => {
                h310
            }
        };
        match m311 {
            Adt1::C0(_, _, _, h309, _, _, _, _) => {
                h309
            }
        }
    }, {
        let m314 = match v1.clone() {
            Adt5::C0(h313, _, _) => {
                h313
            }
        };
        match m314 {
            Adt1::C0(_, _, _, _, h312, _, _, _) => {
                h312
            }
        }
    }, {
        let m317 = match v1.clone() {
            Adt5::C0(h316, _, _) => {
                h316
            }
        };
        match m317 {
            Adt1::C0(_, _, _, _, _, h315, _, _) => {
                h315
            }
        }
    }, {
        let m320 = match v1.clone() {
            Adt5::C0(h319, _, _) => {
                h319
            }
        };
        match m320 {
            Adt1::C0(_, _, _, _, _, _, h318, _) => {
                h318
            }
        }
    }, {
        let m323 = match v1 {
            Adt5::C0(h322, _, _) => {
                h322
            }
        };
        match m323 {
            Adt1::C0(_, _, _, _, _, _, _, h321) => {
                h321
            }
        }
    }), 0u64, 64u32, {
        let a324 = v3;
        let a325 = f52();
        fixed_u32::bit_or(a324, a325)
    }))
}

pub fn f34(v0: Adt7, v1: u64) -> Adt7 {
    Adt7::C0(2u64, match v0.clone() {
        Adt7::C0(_, h326, _, _, _, _, _, _, _, _) => {
            h326
        }
    }, match v0.clone() {
        Adt7::C0(_, _, h327, _, _, _, _, _, _, _) => {
            h327
        }
    }, v1, match v0.clone() {
        Adt7::C0(_, _, _, _, h328, _, _, _, _, _) => {
            h328
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, h329, _, _, _, _) => {
            h329
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, h330, _, _, _) => {
            h330
        }
    }, v1, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, _, h331, _) => {
            h331
        }
    }, {
        let a333 = match v0 {
            Adt7::C0(_, _, _, _, _, _, _, _, _, h332) => {
                h332
            }
        };
        let a334 = f53();
        fixed_u32::bit_or(a333, a334)
    })
}

pub fn f35() -> u32 {
    16u32
}

pub fn f36() -> u32 {
    32u32
}

pub fn f37(v0: Fn7, p338: List<Adt2>, p339: List<Adt2>) -> List<Adt2> {
    let m337 = (p338, p339);
    {
        let mut m337 = m337;
        loop {
            let step = {
                let (v1, v2) = m337;
                let m335 = v2;
                match m335.uncons() {
                    None => {
                        Err::<(List<Adt2>, List<Adt2>), List<Adt2>>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(List<Adt2>, List<Adt2>), List<Adt2>>(({
                            let c336: Fn7 = v0.clone();
                            c336.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m337 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f38(v0: List<Adt2>, v1: Adt2) -> List<Adt2> {
    List::cons(v1, v0)
}

pub fn f39(v0: Adt1, v1: u32, v2: Adt8) -> R<(List<Adt5>, List<Adt2>)> {
    let v3: Adt8 = {
        let (h340, _) = f54(Fn9::F55(v1, v0), 64u64, v2)?;
        h340
    };
    let v4: List<Adt5> = List::cons(match v3.clone() {
        Adt8::C0(h341, _, _, _) => {
            h341
        }
    }, match v3.clone() {
        Adt8::C0(_, _, h342, _) => {
            h342
        }
    });
    Ok((v4.clone(), List::cons(Adt2::C1(match v3.clone() {
        Adt8::C0(h343, _, _, _) => {
            h343
        }
    }, {
        let a344 = v4;
        length_list(a344)
    }), match v3 {
        Adt8::C0(_, _, _, h345) => {
            h345
        }
    })))
}

pub fn f40(v0: Adt1, v1: u32) -> Adt6 {
    Adt6::C0(v0, v1, 0u64, Bytes::lit(&[]), List::<Adt5>::nil(), List::<Adt2>::nil(), List::<Adt2>::nil())
}

pub fn f41() -> Adt1 {
    Adt1::C0(1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, 1359893119u32, 2600822924u32, 528734635u32, 1541459225u32)
}

pub fn f42(v0: Fn8, p349: Adt4, p350: List<Adt2>) -> Adt4 {
    let m348 = (p349, p350);
    {
        let mut m348 = m348;
        loop {
            let step = {
                let (v1, v2) = m348;
                let m346 = v2;
                match m346.uncons() {
                    None => {
                        Err::<(Adt4, List<Adt2>), Adt4>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt4, List<Adt2>), Adt4>(({
                            let c347: Fn8 = v0.clone();
                            c347.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m348 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f43(v0: u64, v1: Adt4, v2: Adt2) -> Adt4 {
    let m351 = v2;
    match m351 {
        Adt2::C0(v3) => {
            let a353 = match v3.clone() {
                Adt3::C0(h352, _, _, _, _, _, _, _, _, _, _, _) => {
                    h352
                }
            };
            let a354 = 2u64;
            if nat_eq(a353, a354) {
                let a356 = match v3.clone() {
                    Adt3::C0(_, _, _, h355, _, _, _, _, _, _, _, _) => {
                        h355
                    }
                };
                let a357 = v0;
                if nat_eq(a356, a357) {
                    match v3 {
                        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h358) => {
                            h358
                        }
                    }
                } else {
                    v1
                }
            } else {
                v1
            }
        }
        Adt2::C1(_, _) => {
            v1
        }
        Adt2::C2(_, _) => {
            v1
        }
    }
}

pub fn f44(v0: u32) -> Bytes {
    let a369 = f56({
        let a359 = v0;
        let a360 = 255u32;
        fixed_u32::bit_and(a359, a360)
    });
    let a370 = {
        let a367 = f56({
            let a361 = f57(v0, 8u32);
            let a362 = 255u32;
            fixed_u32::bit_and(a361, a362)
        });
        let a368 = {
            let a365 = f56({
                let a363 = f57(v0, 16u32);
                let a364 = 255u32;
                fixed_u32::bit_and(a363, a364)
            });
            let a366 = f56(f57(v0, 24u32));
            append_bytes(a365, a366)
        };
        append_bytes(a367, a368)
    };
    append_bytes(a369, a370)
}

pub fn f45(v0: List<Bytes>) -> List<Bytes> {
    let v1: (Option<Bytes>, List<Bytes>) = f58(Fn10::F59, (None::<Bytes>, List::<Bytes>::nil()), v0);
    let v3: List<Bytes> = {
        let m372 = {
            let (h371, _) = v1;
            h371
        };
        match m372 {
            None => {
                let (_, h374) = v1;
                h374
            }
            Some(v2) => {
                List::cons(v2, {
                    let (_, h373) = v1;
                    h373
                })
            }
        }
    };
    f8(Fn1::F60, List::<Bytes>::nil(), v3)
}

pub fn f46(v0: Bytes, v1: u64, v2: u64) -> R<Adt4> {
    Ok(Adt4::C0(f61(v0.clone(), v1, {
        let a375 = v2;
        let a376 = 0u64;
        nat_add(a375, a376)?
    })?, f61(v0.clone(), v1, {
        let a377 = v2;
        let a378 = 4u64;
        nat_add(a377, a378)?
    })?, f61(v0.clone(), v1, {
        let a379 = v2;
        let a380 = 8u64;
        nat_add(a379, a380)?
    })?, f61(v0.clone(), v1, {
        let a381 = v2;
        let a382 = 12u64;
        nat_add(a381, a382)?
    })?, f61(v0.clone(), v1, {
        let a383 = v2;
        let a384 = 16u64;
        nat_add(a383, a384)?
    })?, f61(v0.clone(), v1, {
        let a385 = v2;
        let a386 = 20u64;
        nat_add(a385, a386)?
    })?, f61(v0.clone(), v1, {
        let a387 = v2;
        let a388 = 24u64;
        nat_add(a387, a388)?
    })?, f61(v0.clone(), v1, {
        let a389 = v2;
        let a390 = 28u64;
        nat_add(a389, a390)?
    })?, f61(v0.clone(), v1, {
        let a391 = v2;
        let a392 = 32u64;
        nat_add(a391, a392)?
    })?, f61(v0.clone(), v1, {
        let a393 = v2;
        let a394 = 36u64;
        nat_add(a393, a394)?
    })?, f61(v0.clone(), v1, {
        let a395 = v2;
        let a396 = 40u64;
        nat_add(a395, a396)?
    })?, f61(v0.clone(), v1, {
        let a397 = v2;
        let a398 = 44u64;
        nat_add(a397, a398)?
    })?, f61(v0.clone(), v1, {
        let a399 = v2;
        let a400 = 48u64;
        nat_add(a399, a400)?
    })?, f61(v0.clone(), v1, {
        let a401 = v2;
        let a402 = 52u64;
        nat_add(a401, a402)?
    })?, f61(v0.clone(), v1, {
        let a403 = v2;
        let a404 = 56u64;
        nat_add(a403, a404)?
    })?, f61(v0, v1, {
        let a405 = v2;
        let a406 = 60u64;
        nat_add(a405, a406)?
    })?))
}

pub fn f47(v0: u64, v1: u64) -> u32 {
    let (_, h407) = {
        let (_, h408) = f62(Fn11::F63, v1, (v0, (1u32, 0u32)));
        h408
    };
    h407
}

pub fn f48() -> u32 {
    1u32
}

pub fn f49(v0: u64) -> u32 {
    f47({
        let a409 = v0;
        let a410 = 4294967296u64;
        let a411 = 0u64;
        nat_rem(a409, a410, a411)
    }, 32u64)
}

pub fn f50(v0: u64) -> u32 {
    let a412 = v0;
    let a413 = 4294967296u64;
    if nat_lt(a412, a413) {
        0u32
    } else {
        f47({
            let a414 = v0;
            let a415 = 4294967296u64;
            let a416 = 0u64;
            nat_quot(a414, a415, a416)
        }, 32u64)
    }
}

pub fn f51(v0: Adt1, v1: Adt4, v2: u32, v3: u32, v4: u32, v5: u32) -> R<Adt4> {
    Ok(f64(f65(v0, v1, v2, v3, v4, v5)?))
}

pub fn f52() -> u32 {
    4u32
}

pub fn f53() -> u32 {
    8u32
}

pub fn f54(v0: Fn9, p421: u64, p422: Adt8) -> R<(Adt8, bool)> {
    let m420 = (p421, p422);
    {
        let mut m420 = m420;
        loop {
            let step = {
                let (v1, v2) = m420;
                let m417 = v1;
                if m417 == 0 {
                    Err::<(u64, Adt8), R<(Adt8, bool)>>(Ok((v2, false)))
                } else {
                    let v3 = m417 - 1;
                    let m419 = {
                        let c418: Fn9 = v0.clone();
                        c418.apply(v2.clone())?
                    };
                    match m419 {
                        None => {
                            Err::<(u64, Adt8), R<(Adt8, bool)>>(Ok((v2, true)))
                        }
                        Some(v4) => {
                            Ok::<(u64, Adt8), R<(Adt8, bool)>>((v3, v4))
                        }
                    }
                }
            };
            match step {
                Ok(next) => { m420 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f55(v0: u32, v1: Adt1, v2: Adt8) -> R<Option<Adt8>> {
    f66(v1, v0, v2)
}

pub fn f56(v0: u32) -> Bytes {
    let a425 = {
        let a423 = v0;
        let a424 = 128u32;
        fixed_u32::bit_and(a423, a424)
    };
    let a426 = 0u32;
    if equal(a425, a426) {
        let a429 = {
            let a427 = v0;
            let a428 = 64u32;
            fixed_u32::bit_and(a427, a428)
        };
        let a430 = 0u32;
        if equal(a429, a430) {
            let a433 = {
                let a431 = v0;
                let a432 = 32u32;
                fixed_u32::bit_and(a431, a432)
            };
            let a434 = 0u32;
            if equal(a433, a434) {
                let a437 = {
                    let a435 = v0;
                    let a436 = 16u32;
                    fixed_u32::bit_and(a435, a436)
                };
                let a438 = 0u32;
                if equal(a437, a438) {
                    let a441 = {
                        let a439 = v0;
                        let a440 = 8u32;
                        fixed_u32::bit_and(a439, a440)
                    };
                    let a442 = 0u32;
                    if equal(a441, a442) {
                        let a445 = {
                            let a443 = v0;
                            let a444 = 4u32;
                            fixed_u32::bit_and(a443, a444)
                        };
                        let a446 = 0u32;
                        if equal(a445, a446) {
                            let a449 = {
                                let a447 = v0;
                                let a448 = 2u32;
                                fixed_u32::bit_and(a447, a448)
                            };
                            let a450 = 0u32;
                            if equal(a449, a450) {
                                let a453 = {
                                    let a451 = v0;
                                    let a452 = 1u32;
                                    fixed_u32::bit_and(a451, a452)
                                };
                                let a454 = 0u32;
                                if equal(a453, a454) {
                                    Bytes::lit(&[0x00])
                                } else {
                                    Bytes::lit(&[0x01])
                                }
                            } else {
                                let a457 = {
                                    let a455 = v0;
                                    let a456 = 1u32;
                                    fixed_u32::bit_and(a455, a456)
                                };
                                let a458 = 0u32;
                                if equal(a457, a458) {
                                    Bytes::lit(&[0x02])
                                } else {
                                    Bytes::lit(&[0x03])
                                }
                            }
                        } else {
                            let a461 = {
                                let a459 = v0;
                                let a460 = 2u32;
                                fixed_u32::bit_and(a459, a460)
                            };
                            let a462 = 0u32;
                            if equal(a461, a462) {
                                let a465 = {
                                    let a463 = v0;
                                    let a464 = 1u32;
                                    fixed_u32::bit_and(a463, a464)
                                };
                                let a466 = 0u32;
                                if equal(a465, a466) {
                                    Bytes::lit(&[0x04])
                                } else {
                                    Bytes::lit(&[0x05])
                                }
                            } else {
                                let a469 = {
                                    let a467 = v0;
                                    let a468 = 1u32;
                                    fixed_u32::bit_and(a467, a468)
                                };
                                let a470 = 0u32;
                                if equal(a469, a470) {
                                    Bytes::lit(&[0x06])
                                } else {
                                    Bytes::lit(&[0x07])
                                }
                            }
                        }
                    } else {
                        let a473 = {
                            let a471 = v0;
                            let a472 = 4u32;
                            fixed_u32::bit_and(a471, a472)
                        };
                        let a474 = 0u32;
                        if equal(a473, a474) {
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
                                    Bytes::lit(&[0x08])
                                } else {
                                    Bytes::lit(&[0x09])
                                }
                            } else {
                                let a485 = {
                                    let a483 = v0;
                                    let a484 = 1u32;
                                    fixed_u32::bit_and(a483, a484)
                                };
                                let a486 = 0u32;
                                if equal(a485, a486) {
                                    Bytes::lit(&[0x0a])
                                } else {
                                    Bytes::lit(&[0x0b])
                                }
                            }
                        } else {
                            let a489 = {
                                let a487 = v0;
                                let a488 = 2u32;
                                fixed_u32::bit_and(a487, a488)
                            };
                            let a490 = 0u32;
                            if equal(a489, a490) {
                                let a493 = {
                                    let a491 = v0;
                                    let a492 = 1u32;
                                    fixed_u32::bit_and(a491, a492)
                                };
                                let a494 = 0u32;
                                if equal(a493, a494) {
                                    Bytes::lit(&[0x0c])
                                } else {
                                    Bytes::lit(&[0x0d])
                                }
                            } else {
                                let a497 = {
                                    let a495 = v0;
                                    let a496 = 1u32;
                                    fixed_u32::bit_and(a495, a496)
                                };
                                let a498 = 0u32;
                                if equal(a497, a498) {
                                    Bytes::lit(&[0x0e])
                                } else {
                                    Bytes::lit(&[0x0f])
                                }
                            }
                        }
                    }
                } else {
                    let a501 = {
                        let a499 = v0;
                        let a500 = 8u32;
                        fixed_u32::bit_and(a499, a500)
                    };
                    let a502 = 0u32;
                    if equal(a501, a502) {
                        let a505 = {
                            let a503 = v0;
                            let a504 = 4u32;
                            fixed_u32::bit_and(a503, a504)
                        };
                        let a506 = 0u32;
                        if equal(a505, a506) {
                            let a509 = {
                                let a507 = v0;
                                let a508 = 2u32;
                                fixed_u32::bit_and(a507, a508)
                            };
                            let a510 = 0u32;
                            if equal(a509, a510) {
                                let a513 = {
                                    let a511 = v0;
                                    let a512 = 1u32;
                                    fixed_u32::bit_and(a511, a512)
                                };
                                let a514 = 0u32;
                                if equal(a513, a514) {
                                    Bytes::lit(&[0x10])
                                } else {
                                    Bytes::lit(&[0x11])
                                }
                            } else {
                                let a517 = {
                                    let a515 = v0;
                                    let a516 = 1u32;
                                    fixed_u32::bit_and(a515, a516)
                                };
                                let a518 = 0u32;
                                if equal(a517, a518) {
                                    Bytes::lit(&[0x12])
                                } else {
                                    Bytes::lit(&[0x13])
                                }
                            }
                        } else {
                            let a521 = {
                                let a519 = v0;
                                let a520 = 2u32;
                                fixed_u32::bit_and(a519, a520)
                            };
                            let a522 = 0u32;
                            if equal(a521, a522) {
                                let a525 = {
                                    let a523 = v0;
                                    let a524 = 1u32;
                                    fixed_u32::bit_and(a523, a524)
                                };
                                let a526 = 0u32;
                                if equal(a525, a526) {
                                    Bytes::lit(&[0x14])
                                } else {
                                    Bytes::lit(&[0x15])
                                }
                            } else {
                                let a529 = {
                                    let a527 = v0;
                                    let a528 = 1u32;
                                    fixed_u32::bit_and(a527, a528)
                                };
                                let a530 = 0u32;
                                if equal(a529, a530) {
                                    Bytes::lit(&[0x16])
                                } else {
                                    Bytes::lit(&[0x17])
                                }
                            }
                        }
                    } else {
                        let a533 = {
                            let a531 = v0;
                            let a532 = 4u32;
                            fixed_u32::bit_and(a531, a532)
                        };
                        let a534 = 0u32;
                        if equal(a533, a534) {
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
                                    Bytes::lit(&[0x18])
                                } else {
                                    Bytes::lit(&[0x19])
                                }
                            } else {
                                let a545 = {
                                    let a543 = v0;
                                    let a544 = 1u32;
                                    fixed_u32::bit_and(a543, a544)
                                };
                                let a546 = 0u32;
                                if equal(a545, a546) {
                                    Bytes::lit(&[0x1a])
                                } else {
                                    Bytes::lit(&[0x1b])
                                }
                            }
                        } else {
                            let a549 = {
                                let a547 = v0;
                                let a548 = 2u32;
                                fixed_u32::bit_and(a547, a548)
                            };
                            let a550 = 0u32;
                            if equal(a549, a550) {
                                let a553 = {
                                    let a551 = v0;
                                    let a552 = 1u32;
                                    fixed_u32::bit_and(a551, a552)
                                };
                                let a554 = 0u32;
                                if equal(a553, a554) {
                                    Bytes::lit(&[0x1c])
                                } else {
                                    Bytes::lit(&[0x1d])
                                }
                            } else {
                                let a557 = {
                                    let a555 = v0;
                                    let a556 = 1u32;
                                    fixed_u32::bit_and(a555, a556)
                                };
                                let a558 = 0u32;
                                if equal(a557, a558) {
                                    Bytes::lit(&[0x1e])
                                } else {
                                    Bytes::lit(&[0x1f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a561 = {
                    let a559 = v0;
                    let a560 = 16u32;
                    fixed_u32::bit_and(a559, a560)
                };
                let a562 = 0u32;
                if equal(a561, a562) {
                    let a565 = {
                        let a563 = v0;
                        let a564 = 8u32;
                        fixed_u32::bit_and(a563, a564)
                    };
                    let a566 = 0u32;
                    if equal(a565, a566) {
                        let a569 = {
                            let a567 = v0;
                            let a568 = 4u32;
                            fixed_u32::bit_and(a567, a568)
                        };
                        let a570 = 0u32;
                        if equal(a569, a570) {
                            let a573 = {
                                let a571 = v0;
                                let a572 = 2u32;
                                fixed_u32::bit_and(a571, a572)
                            };
                            let a574 = 0u32;
                            if equal(a573, a574) {
                                let a577 = {
                                    let a575 = v0;
                                    let a576 = 1u32;
                                    fixed_u32::bit_and(a575, a576)
                                };
                                let a578 = 0u32;
                                if equal(a577, a578) {
                                    Bytes::lit(&[0x20])
                                } else {
                                    Bytes::lit(&[0x21])
                                }
                            } else {
                                let a581 = {
                                    let a579 = v0;
                                    let a580 = 1u32;
                                    fixed_u32::bit_and(a579, a580)
                                };
                                let a582 = 0u32;
                                if equal(a581, a582) {
                                    Bytes::lit(&[0x22])
                                } else {
                                    Bytes::lit(&[0x23])
                                }
                            }
                        } else {
                            let a585 = {
                                let a583 = v0;
                                let a584 = 2u32;
                                fixed_u32::bit_and(a583, a584)
                            };
                            let a586 = 0u32;
                            if equal(a585, a586) {
                                let a589 = {
                                    let a587 = v0;
                                    let a588 = 1u32;
                                    fixed_u32::bit_and(a587, a588)
                                };
                                let a590 = 0u32;
                                if equal(a589, a590) {
                                    Bytes::lit(&[0x24])
                                } else {
                                    Bytes::lit(&[0x25])
                                }
                            } else {
                                let a593 = {
                                    let a591 = v0;
                                    let a592 = 1u32;
                                    fixed_u32::bit_and(a591, a592)
                                };
                                let a594 = 0u32;
                                if equal(a593, a594) {
                                    Bytes::lit(&[0x26])
                                } else {
                                    Bytes::lit(&[0x27])
                                }
                            }
                        }
                    } else {
                        let a597 = {
                            let a595 = v0;
                            let a596 = 4u32;
                            fixed_u32::bit_and(a595, a596)
                        };
                        let a598 = 0u32;
                        if equal(a597, a598) {
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
                                    Bytes::lit(&[0x28])
                                } else {
                                    Bytes::lit(&[0x29])
                                }
                            } else {
                                let a609 = {
                                    let a607 = v0;
                                    let a608 = 1u32;
                                    fixed_u32::bit_and(a607, a608)
                                };
                                let a610 = 0u32;
                                if equal(a609, a610) {
                                    Bytes::lit(&[0x2a])
                                } else {
                                    Bytes::lit(&[0x2b])
                                }
                            }
                        } else {
                            let a613 = {
                                let a611 = v0;
                                let a612 = 2u32;
                                fixed_u32::bit_and(a611, a612)
                            };
                            let a614 = 0u32;
                            if equal(a613, a614) {
                                let a617 = {
                                    let a615 = v0;
                                    let a616 = 1u32;
                                    fixed_u32::bit_and(a615, a616)
                                };
                                let a618 = 0u32;
                                if equal(a617, a618) {
                                    Bytes::lit(&[0x2c])
                                } else {
                                    Bytes::lit(&[0x2d])
                                }
                            } else {
                                let a621 = {
                                    let a619 = v0;
                                    let a620 = 1u32;
                                    fixed_u32::bit_and(a619, a620)
                                };
                                let a622 = 0u32;
                                if equal(a621, a622) {
                                    Bytes::lit(&[0x2e])
                                } else {
                                    Bytes::lit(&[0x2f])
                                }
                            }
                        }
                    }
                } else {
                    let a625 = {
                        let a623 = v0;
                        let a624 = 8u32;
                        fixed_u32::bit_and(a623, a624)
                    };
                    let a626 = 0u32;
                    if equal(a625, a626) {
                        let a629 = {
                            let a627 = v0;
                            let a628 = 4u32;
                            fixed_u32::bit_and(a627, a628)
                        };
                        let a630 = 0u32;
                        if equal(a629, a630) {
                            let a633 = {
                                let a631 = v0;
                                let a632 = 2u32;
                                fixed_u32::bit_and(a631, a632)
                            };
                            let a634 = 0u32;
                            if equal(a633, a634) {
                                let a637 = {
                                    let a635 = v0;
                                    let a636 = 1u32;
                                    fixed_u32::bit_and(a635, a636)
                                };
                                let a638 = 0u32;
                                if equal(a637, a638) {
                                    Bytes::lit(&[0x30])
                                } else {
                                    Bytes::lit(&[0x31])
                                }
                            } else {
                                let a641 = {
                                    let a639 = v0;
                                    let a640 = 1u32;
                                    fixed_u32::bit_and(a639, a640)
                                };
                                let a642 = 0u32;
                                if equal(a641, a642) {
                                    Bytes::lit(&[0x32])
                                } else {
                                    Bytes::lit(&[0x33])
                                }
                            }
                        } else {
                            let a645 = {
                                let a643 = v0;
                                let a644 = 2u32;
                                fixed_u32::bit_and(a643, a644)
                            };
                            let a646 = 0u32;
                            if equal(a645, a646) {
                                let a649 = {
                                    let a647 = v0;
                                    let a648 = 1u32;
                                    fixed_u32::bit_and(a647, a648)
                                };
                                let a650 = 0u32;
                                if equal(a649, a650) {
                                    Bytes::lit(&[0x34])
                                } else {
                                    Bytes::lit(&[0x35])
                                }
                            } else {
                                let a653 = {
                                    let a651 = v0;
                                    let a652 = 1u32;
                                    fixed_u32::bit_and(a651, a652)
                                };
                                let a654 = 0u32;
                                if equal(a653, a654) {
                                    Bytes::lit(&[0x36])
                                } else {
                                    Bytes::lit(&[0x37])
                                }
                            }
                        }
                    } else {
                        let a657 = {
                            let a655 = v0;
                            let a656 = 4u32;
                            fixed_u32::bit_and(a655, a656)
                        };
                        let a658 = 0u32;
                        if equal(a657, a658) {
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
                                    Bytes::lit(&[0x38])
                                } else {
                                    Bytes::lit(&[0x39])
                                }
                            } else {
                                let a669 = {
                                    let a667 = v0;
                                    let a668 = 1u32;
                                    fixed_u32::bit_and(a667, a668)
                                };
                                let a670 = 0u32;
                                if equal(a669, a670) {
                                    Bytes::lit(&[0x3a])
                                } else {
                                    Bytes::lit(&[0x3b])
                                }
                            }
                        } else {
                            let a673 = {
                                let a671 = v0;
                                let a672 = 2u32;
                                fixed_u32::bit_and(a671, a672)
                            };
                            let a674 = 0u32;
                            if equal(a673, a674) {
                                let a677 = {
                                    let a675 = v0;
                                    let a676 = 1u32;
                                    fixed_u32::bit_and(a675, a676)
                                };
                                let a678 = 0u32;
                                if equal(a677, a678) {
                                    Bytes::lit(&[0x3c])
                                } else {
                                    Bytes::lit(&[0x3d])
                                }
                            } else {
                                let a681 = {
                                    let a679 = v0;
                                    let a680 = 1u32;
                                    fixed_u32::bit_and(a679, a680)
                                };
                                let a682 = 0u32;
                                if equal(a681, a682) {
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
            let a685 = {
                let a683 = v0;
                let a684 = 32u32;
                fixed_u32::bit_and(a683, a684)
            };
            let a686 = 0u32;
            if equal(a685, a686) {
                let a689 = {
                    let a687 = v0;
                    let a688 = 16u32;
                    fixed_u32::bit_and(a687, a688)
                };
                let a690 = 0u32;
                if equal(a689, a690) {
                    let a693 = {
                        let a691 = v0;
                        let a692 = 8u32;
                        fixed_u32::bit_and(a691, a692)
                    };
                    let a694 = 0u32;
                    if equal(a693, a694) {
                        let a697 = {
                            let a695 = v0;
                            let a696 = 4u32;
                            fixed_u32::bit_and(a695, a696)
                        };
                        let a698 = 0u32;
                        if equal(a697, a698) {
                            let a701 = {
                                let a699 = v0;
                                let a700 = 2u32;
                                fixed_u32::bit_and(a699, a700)
                            };
                            let a702 = 0u32;
                            if equal(a701, a702) {
                                let a705 = {
                                    let a703 = v0;
                                    let a704 = 1u32;
                                    fixed_u32::bit_and(a703, a704)
                                };
                                let a706 = 0u32;
                                if equal(a705, a706) {
                                    Bytes::lit(&[0x40])
                                } else {
                                    Bytes::lit(&[0x41])
                                }
                            } else {
                                let a709 = {
                                    let a707 = v0;
                                    let a708 = 1u32;
                                    fixed_u32::bit_and(a707, a708)
                                };
                                let a710 = 0u32;
                                if equal(a709, a710) {
                                    Bytes::lit(&[0x42])
                                } else {
                                    Bytes::lit(&[0x43])
                                }
                            }
                        } else {
                            let a713 = {
                                let a711 = v0;
                                let a712 = 2u32;
                                fixed_u32::bit_and(a711, a712)
                            };
                            let a714 = 0u32;
                            if equal(a713, a714) {
                                let a717 = {
                                    let a715 = v0;
                                    let a716 = 1u32;
                                    fixed_u32::bit_and(a715, a716)
                                };
                                let a718 = 0u32;
                                if equal(a717, a718) {
                                    Bytes::lit(&[0x44])
                                } else {
                                    Bytes::lit(&[0x45])
                                }
                            } else {
                                let a721 = {
                                    let a719 = v0;
                                    let a720 = 1u32;
                                    fixed_u32::bit_and(a719, a720)
                                };
                                let a722 = 0u32;
                                if equal(a721, a722) {
                                    Bytes::lit(&[0x46])
                                } else {
                                    Bytes::lit(&[0x47])
                                }
                            }
                        }
                    } else {
                        let a725 = {
                            let a723 = v0;
                            let a724 = 4u32;
                            fixed_u32::bit_and(a723, a724)
                        };
                        let a726 = 0u32;
                        if equal(a725, a726) {
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
                                    Bytes::lit(&[0x48])
                                } else {
                                    Bytes::lit(&[0x49])
                                }
                            } else {
                                let a737 = {
                                    let a735 = v0;
                                    let a736 = 1u32;
                                    fixed_u32::bit_and(a735, a736)
                                };
                                let a738 = 0u32;
                                if equal(a737, a738) {
                                    Bytes::lit(&[0x4a])
                                } else {
                                    Bytes::lit(&[0x4b])
                                }
                            }
                        } else {
                            let a741 = {
                                let a739 = v0;
                                let a740 = 2u32;
                                fixed_u32::bit_and(a739, a740)
                            };
                            let a742 = 0u32;
                            if equal(a741, a742) {
                                let a745 = {
                                    let a743 = v0;
                                    let a744 = 1u32;
                                    fixed_u32::bit_and(a743, a744)
                                };
                                let a746 = 0u32;
                                if equal(a745, a746) {
                                    Bytes::lit(&[0x4c])
                                } else {
                                    Bytes::lit(&[0x4d])
                                }
                            } else {
                                let a749 = {
                                    let a747 = v0;
                                    let a748 = 1u32;
                                    fixed_u32::bit_and(a747, a748)
                                };
                                let a750 = 0u32;
                                if equal(a749, a750) {
                                    Bytes::lit(&[0x4e])
                                } else {
                                    Bytes::lit(&[0x4f])
                                }
                            }
                        }
                    }
                } else {
                    let a753 = {
                        let a751 = v0;
                        let a752 = 8u32;
                        fixed_u32::bit_and(a751, a752)
                    };
                    let a754 = 0u32;
                    if equal(a753, a754) {
                        let a757 = {
                            let a755 = v0;
                            let a756 = 4u32;
                            fixed_u32::bit_and(a755, a756)
                        };
                        let a758 = 0u32;
                        if equal(a757, a758) {
                            let a761 = {
                                let a759 = v0;
                                let a760 = 2u32;
                                fixed_u32::bit_and(a759, a760)
                            };
                            let a762 = 0u32;
                            if equal(a761, a762) {
                                let a765 = {
                                    let a763 = v0;
                                    let a764 = 1u32;
                                    fixed_u32::bit_and(a763, a764)
                                };
                                let a766 = 0u32;
                                if equal(a765, a766) {
                                    Bytes::lit(&[0x50])
                                } else {
                                    Bytes::lit(&[0x51])
                                }
                            } else {
                                let a769 = {
                                    let a767 = v0;
                                    let a768 = 1u32;
                                    fixed_u32::bit_and(a767, a768)
                                };
                                let a770 = 0u32;
                                if equal(a769, a770) {
                                    Bytes::lit(&[0x52])
                                } else {
                                    Bytes::lit(&[0x53])
                                }
                            }
                        } else {
                            let a773 = {
                                let a771 = v0;
                                let a772 = 2u32;
                                fixed_u32::bit_and(a771, a772)
                            };
                            let a774 = 0u32;
                            if equal(a773, a774) {
                                let a777 = {
                                    let a775 = v0;
                                    let a776 = 1u32;
                                    fixed_u32::bit_and(a775, a776)
                                };
                                let a778 = 0u32;
                                if equal(a777, a778) {
                                    Bytes::lit(&[0x54])
                                } else {
                                    Bytes::lit(&[0x55])
                                }
                            } else {
                                let a781 = {
                                    let a779 = v0;
                                    let a780 = 1u32;
                                    fixed_u32::bit_and(a779, a780)
                                };
                                let a782 = 0u32;
                                if equal(a781, a782) {
                                    Bytes::lit(&[0x56])
                                } else {
                                    Bytes::lit(&[0x57])
                                }
                            }
                        }
                    } else {
                        let a785 = {
                            let a783 = v0;
                            let a784 = 4u32;
                            fixed_u32::bit_and(a783, a784)
                        };
                        let a786 = 0u32;
                        if equal(a785, a786) {
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
                                    Bytes::lit(&[0x58])
                                } else {
                                    Bytes::lit(&[0x59])
                                }
                            } else {
                                let a797 = {
                                    let a795 = v0;
                                    let a796 = 1u32;
                                    fixed_u32::bit_and(a795, a796)
                                };
                                let a798 = 0u32;
                                if equal(a797, a798) {
                                    Bytes::lit(&[0x5a])
                                } else {
                                    Bytes::lit(&[0x5b])
                                }
                            }
                        } else {
                            let a801 = {
                                let a799 = v0;
                                let a800 = 2u32;
                                fixed_u32::bit_and(a799, a800)
                            };
                            let a802 = 0u32;
                            if equal(a801, a802) {
                                let a805 = {
                                    let a803 = v0;
                                    let a804 = 1u32;
                                    fixed_u32::bit_and(a803, a804)
                                };
                                let a806 = 0u32;
                                if equal(a805, a806) {
                                    Bytes::lit(&[0x5c])
                                } else {
                                    Bytes::lit(&[0x5d])
                                }
                            } else {
                                let a809 = {
                                    let a807 = v0;
                                    let a808 = 1u32;
                                    fixed_u32::bit_and(a807, a808)
                                };
                                let a810 = 0u32;
                                if equal(a809, a810) {
                                    Bytes::lit(&[0x5e])
                                } else {
                                    Bytes::lit(&[0x5f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a813 = {
                    let a811 = v0;
                    let a812 = 16u32;
                    fixed_u32::bit_and(a811, a812)
                };
                let a814 = 0u32;
                if equal(a813, a814) {
                    let a817 = {
                        let a815 = v0;
                        let a816 = 8u32;
                        fixed_u32::bit_and(a815, a816)
                    };
                    let a818 = 0u32;
                    if equal(a817, a818) {
                        let a821 = {
                            let a819 = v0;
                            let a820 = 4u32;
                            fixed_u32::bit_and(a819, a820)
                        };
                        let a822 = 0u32;
                        if equal(a821, a822) {
                            let a825 = {
                                let a823 = v0;
                                let a824 = 2u32;
                                fixed_u32::bit_and(a823, a824)
                            };
                            let a826 = 0u32;
                            if equal(a825, a826) {
                                let a829 = {
                                    let a827 = v0;
                                    let a828 = 1u32;
                                    fixed_u32::bit_and(a827, a828)
                                };
                                let a830 = 0u32;
                                if equal(a829, a830) {
                                    Bytes::lit(&[0x60])
                                } else {
                                    Bytes::lit(&[0x61])
                                }
                            } else {
                                let a833 = {
                                    let a831 = v0;
                                    let a832 = 1u32;
                                    fixed_u32::bit_and(a831, a832)
                                };
                                let a834 = 0u32;
                                if equal(a833, a834) {
                                    Bytes::lit(&[0x62])
                                } else {
                                    Bytes::lit(&[0x63])
                                }
                            }
                        } else {
                            let a837 = {
                                let a835 = v0;
                                let a836 = 2u32;
                                fixed_u32::bit_and(a835, a836)
                            };
                            let a838 = 0u32;
                            if equal(a837, a838) {
                                let a841 = {
                                    let a839 = v0;
                                    let a840 = 1u32;
                                    fixed_u32::bit_and(a839, a840)
                                };
                                let a842 = 0u32;
                                if equal(a841, a842) {
                                    Bytes::lit(&[0x64])
                                } else {
                                    Bytes::lit(&[0x65])
                                }
                            } else {
                                let a845 = {
                                    let a843 = v0;
                                    let a844 = 1u32;
                                    fixed_u32::bit_and(a843, a844)
                                };
                                let a846 = 0u32;
                                if equal(a845, a846) {
                                    Bytes::lit(&[0x66])
                                } else {
                                    Bytes::lit(&[0x67])
                                }
                            }
                        }
                    } else {
                        let a849 = {
                            let a847 = v0;
                            let a848 = 4u32;
                            fixed_u32::bit_and(a847, a848)
                        };
                        let a850 = 0u32;
                        if equal(a849, a850) {
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
                                    Bytes::lit(&[0x68])
                                } else {
                                    Bytes::lit(&[0x69])
                                }
                            } else {
                                let a861 = {
                                    let a859 = v0;
                                    let a860 = 1u32;
                                    fixed_u32::bit_and(a859, a860)
                                };
                                let a862 = 0u32;
                                if equal(a861, a862) {
                                    Bytes::lit(&[0x6a])
                                } else {
                                    Bytes::lit(&[0x6b])
                                }
                            }
                        } else {
                            let a865 = {
                                let a863 = v0;
                                let a864 = 2u32;
                                fixed_u32::bit_and(a863, a864)
                            };
                            let a866 = 0u32;
                            if equal(a865, a866) {
                                let a869 = {
                                    let a867 = v0;
                                    let a868 = 1u32;
                                    fixed_u32::bit_and(a867, a868)
                                };
                                let a870 = 0u32;
                                if equal(a869, a870) {
                                    Bytes::lit(&[0x6c])
                                } else {
                                    Bytes::lit(&[0x6d])
                                }
                            } else {
                                let a873 = {
                                    let a871 = v0;
                                    let a872 = 1u32;
                                    fixed_u32::bit_and(a871, a872)
                                };
                                let a874 = 0u32;
                                if equal(a873, a874) {
                                    Bytes::lit(&[0x6e])
                                } else {
                                    Bytes::lit(&[0x6f])
                                }
                            }
                        }
                    }
                } else {
                    let a877 = {
                        let a875 = v0;
                        let a876 = 8u32;
                        fixed_u32::bit_and(a875, a876)
                    };
                    let a878 = 0u32;
                    if equal(a877, a878) {
                        let a881 = {
                            let a879 = v0;
                            let a880 = 4u32;
                            fixed_u32::bit_and(a879, a880)
                        };
                        let a882 = 0u32;
                        if equal(a881, a882) {
                            let a885 = {
                                let a883 = v0;
                                let a884 = 2u32;
                                fixed_u32::bit_and(a883, a884)
                            };
                            let a886 = 0u32;
                            if equal(a885, a886) {
                                let a889 = {
                                    let a887 = v0;
                                    let a888 = 1u32;
                                    fixed_u32::bit_and(a887, a888)
                                };
                                let a890 = 0u32;
                                if equal(a889, a890) {
                                    Bytes::lit(&[0x70])
                                } else {
                                    Bytes::lit(&[0x71])
                                }
                            } else {
                                let a893 = {
                                    let a891 = v0;
                                    let a892 = 1u32;
                                    fixed_u32::bit_and(a891, a892)
                                };
                                let a894 = 0u32;
                                if equal(a893, a894) {
                                    Bytes::lit(&[0x72])
                                } else {
                                    Bytes::lit(&[0x73])
                                }
                            }
                        } else {
                            let a897 = {
                                let a895 = v0;
                                let a896 = 2u32;
                                fixed_u32::bit_and(a895, a896)
                            };
                            let a898 = 0u32;
                            if equal(a897, a898) {
                                let a901 = {
                                    let a899 = v0;
                                    let a900 = 1u32;
                                    fixed_u32::bit_and(a899, a900)
                                };
                                let a902 = 0u32;
                                if equal(a901, a902) {
                                    Bytes::lit(&[0x74])
                                } else {
                                    Bytes::lit(&[0x75])
                                }
                            } else {
                                let a905 = {
                                    let a903 = v0;
                                    let a904 = 1u32;
                                    fixed_u32::bit_and(a903, a904)
                                };
                                let a906 = 0u32;
                                if equal(a905, a906) {
                                    Bytes::lit(&[0x76])
                                } else {
                                    Bytes::lit(&[0x77])
                                }
                            }
                        }
                    } else {
                        let a909 = {
                            let a907 = v0;
                            let a908 = 4u32;
                            fixed_u32::bit_and(a907, a908)
                        };
                        let a910 = 0u32;
                        if equal(a909, a910) {
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
                                    Bytes::lit(&[0x78])
                                } else {
                                    Bytes::lit(&[0x79])
                                }
                            } else {
                                let a921 = {
                                    let a919 = v0;
                                    let a920 = 1u32;
                                    fixed_u32::bit_and(a919, a920)
                                };
                                let a922 = 0u32;
                                if equal(a921, a922) {
                                    Bytes::lit(&[0x7a])
                                } else {
                                    Bytes::lit(&[0x7b])
                                }
                            }
                        } else {
                            let a925 = {
                                let a923 = v0;
                                let a924 = 2u32;
                                fixed_u32::bit_and(a923, a924)
                            };
                            let a926 = 0u32;
                            if equal(a925, a926) {
                                let a929 = {
                                    let a927 = v0;
                                    let a928 = 1u32;
                                    fixed_u32::bit_and(a927, a928)
                                };
                                let a930 = 0u32;
                                if equal(a929, a930) {
                                    Bytes::lit(&[0x7c])
                                } else {
                                    Bytes::lit(&[0x7d])
                                }
                            } else {
                                let a933 = {
                                    let a931 = v0;
                                    let a932 = 1u32;
                                    fixed_u32::bit_and(a931, a932)
                                };
                                let a934 = 0u32;
                                if equal(a933, a934) {
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
        let a937 = {
            let a935 = v0;
            let a936 = 64u32;
            fixed_u32::bit_and(a935, a936)
        };
        let a938 = 0u32;
        if equal(a937, a938) {
            let a941 = {
                let a939 = v0;
                let a940 = 32u32;
                fixed_u32::bit_and(a939, a940)
            };
            let a942 = 0u32;
            if equal(a941, a942) {
                let a945 = {
                    let a943 = v0;
                    let a944 = 16u32;
                    fixed_u32::bit_and(a943, a944)
                };
                let a946 = 0u32;
                if equal(a945, a946) {
                    let a949 = {
                        let a947 = v0;
                        let a948 = 8u32;
                        fixed_u32::bit_and(a947, a948)
                    };
                    let a950 = 0u32;
                    if equal(a949, a950) {
                        let a953 = {
                            let a951 = v0;
                            let a952 = 4u32;
                            fixed_u32::bit_and(a951, a952)
                        };
                        let a954 = 0u32;
                        if equal(a953, a954) {
                            let a957 = {
                                let a955 = v0;
                                let a956 = 2u32;
                                fixed_u32::bit_and(a955, a956)
                            };
                            let a958 = 0u32;
                            if equal(a957, a958) {
                                let a961 = {
                                    let a959 = v0;
                                    let a960 = 1u32;
                                    fixed_u32::bit_and(a959, a960)
                                };
                                let a962 = 0u32;
                                if equal(a961, a962) {
                                    Bytes::lit(&[0x80])
                                } else {
                                    Bytes::lit(&[0x81])
                                }
                            } else {
                                let a965 = {
                                    let a963 = v0;
                                    let a964 = 1u32;
                                    fixed_u32::bit_and(a963, a964)
                                };
                                let a966 = 0u32;
                                if equal(a965, a966) {
                                    Bytes::lit(&[0x82])
                                } else {
                                    Bytes::lit(&[0x83])
                                }
                            }
                        } else {
                            let a969 = {
                                let a967 = v0;
                                let a968 = 2u32;
                                fixed_u32::bit_and(a967, a968)
                            };
                            let a970 = 0u32;
                            if equal(a969, a970) {
                                let a973 = {
                                    let a971 = v0;
                                    let a972 = 1u32;
                                    fixed_u32::bit_and(a971, a972)
                                };
                                let a974 = 0u32;
                                if equal(a973, a974) {
                                    Bytes::lit(&[0x84])
                                } else {
                                    Bytes::lit(&[0x85])
                                }
                            } else {
                                let a977 = {
                                    let a975 = v0;
                                    let a976 = 1u32;
                                    fixed_u32::bit_and(a975, a976)
                                };
                                let a978 = 0u32;
                                if equal(a977, a978) {
                                    Bytes::lit(&[0x86])
                                } else {
                                    Bytes::lit(&[0x87])
                                }
                            }
                        }
                    } else {
                        let a981 = {
                            let a979 = v0;
                            let a980 = 4u32;
                            fixed_u32::bit_and(a979, a980)
                        };
                        let a982 = 0u32;
                        if equal(a981, a982) {
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
                                    Bytes::lit(&[0x88])
                                } else {
                                    Bytes::lit(&[0x89])
                                }
                            } else {
                                let a993 = {
                                    let a991 = v0;
                                    let a992 = 1u32;
                                    fixed_u32::bit_and(a991, a992)
                                };
                                let a994 = 0u32;
                                if equal(a993, a994) {
                                    Bytes::lit(&[0x8a])
                                } else {
                                    Bytes::lit(&[0x8b])
                                }
                            }
                        } else {
                            let a997 = {
                                let a995 = v0;
                                let a996 = 2u32;
                                fixed_u32::bit_and(a995, a996)
                            };
                            let a998 = 0u32;
                            if equal(a997, a998) {
                                let a1001 = {
                                    let a999 = v0;
                                    let a1000 = 1u32;
                                    fixed_u32::bit_and(a999, a1000)
                                };
                                let a1002 = 0u32;
                                if equal(a1001, a1002) {
                                    Bytes::lit(&[0x8c])
                                } else {
                                    Bytes::lit(&[0x8d])
                                }
                            } else {
                                let a1005 = {
                                    let a1003 = v0;
                                    let a1004 = 1u32;
                                    fixed_u32::bit_and(a1003, a1004)
                                };
                                let a1006 = 0u32;
                                if equal(a1005, a1006) {
                                    Bytes::lit(&[0x8e])
                                } else {
                                    Bytes::lit(&[0x8f])
                                }
                            }
                        }
                    }
                } else {
                    let a1009 = {
                        let a1007 = v0;
                        let a1008 = 8u32;
                        fixed_u32::bit_and(a1007, a1008)
                    };
                    let a1010 = 0u32;
                    if equal(a1009, a1010) {
                        let a1013 = {
                            let a1011 = v0;
                            let a1012 = 4u32;
                            fixed_u32::bit_and(a1011, a1012)
                        };
                        let a1014 = 0u32;
                        if equal(a1013, a1014) {
                            let a1017 = {
                                let a1015 = v0;
                                let a1016 = 2u32;
                                fixed_u32::bit_and(a1015, a1016)
                            };
                            let a1018 = 0u32;
                            if equal(a1017, a1018) {
                                let a1021 = {
                                    let a1019 = v0;
                                    let a1020 = 1u32;
                                    fixed_u32::bit_and(a1019, a1020)
                                };
                                let a1022 = 0u32;
                                if equal(a1021, a1022) {
                                    Bytes::lit(&[0x90])
                                } else {
                                    Bytes::lit(&[0x91])
                                }
                            } else {
                                let a1025 = {
                                    let a1023 = v0;
                                    let a1024 = 1u32;
                                    fixed_u32::bit_and(a1023, a1024)
                                };
                                let a1026 = 0u32;
                                if equal(a1025, a1026) {
                                    Bytes::lit(&[0x92])
                                } else {
                                    Bytes::lit(&[0x93])
                                }
                            }
                        } else {
                            let a1029 = {
                                let a1027 = v0;
                                let a1028 = 2u32;
                                fixed_u32::bit_and(a1027, a1028)
                            };
                            let a1030 = 0u32;
                            if equal(a1029, a1030) {
                                let a1033 = {
                                    let a1031 = v0;
                                    let a1032 = 1u32;
                                    fixed_u32::bit_and(a1031, a1032)
                                };
                                let a1034 = 0u32;
                                if equal(a1033, a1034) {
                                    Bytes::lit(&[0x94])
                                } else {
                                    Bytes::lit(&[0x95])
                                }
                            } else {
                                let a1037 = {
                                    let a1035 = v0;
                                    let a1036 = 1u32;
                                    fixed_u32::bit_and(a1035, a1036)
                                };
                                let a1038 = 0u32;
                                if equal(a1037, a1038) {
                                    Bytes::lit(&[0x96])
                                } else {
                                    Bytes::lit(&[0x97])
                                }
                            }
                        }
                    } else {
                        let a1041 = {
                            let a1039 = v0;
                            let a1040 = 4u32;
                            fixed_u32::bit_and(a1039, a1040)
                        };
                        let a1042 = 0u32;
                        if equal(a1041, a1042) {
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
                                    Bytes::lit(&[0x98])
                                } else {
                                    Bytes::lit(&[0x99])
                                }
                            } else {
                                let a1053 = {
                                    let a1051 = v0;
                                    let a1052 = 1u32;
                                    fixed_u32::bit_and(a1051, a1052)
                                };
                                let a1054 = 0u32;
                                if equal(a1053, a1054) {
                                    Bytes::lit(&[0x9a])
                                } else {
                                    Bytes::lit(&[0x9b])
                                }
                            }
                        } else {
                            let a1057 = {
                                let a1055 = v0;
                                let a1056 = 2u32;
                                fixed_u32::bit_and(a1055, a1056)
                            };
                            let a1058 = 0u32;
                            if equal(a1057, a1058) {
                                let a1061 = {
                                    let a1059 = v0;
                                    let a1060 = 1u32;
                                    fixed_u32::bit_and(a1059, a1060)
                                };
                                let a1062 = 0u32;
                                if equal(a1061, a1062) {
                                    Bytes::lit(&[0x9c])
                                } else {
                                    Bytes::lit(&[0x9d])
                                }
                            } else {
                                let a1065 = {
                                    let a1063 = v0;
                                    let a1064 = 1u32;
                                    fixed_u32::bit_and(a1063, a1064)
                                };
                                let a1066 = 0u32;
                                if equal(a1065, a1066) {
                                    Bytes::lit(&[0x9e])
                                } else {
                                    Bytes::lit(&[0x9f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a1069 = {
                    let a1067 = v0;
                    let a1068 = 16u32;
                    fixed_u32::bit_and(a1067, a1068)
                };
                let a1070 = 0u32;
                if equal(a1069, a1070) {
                    let a1073 = {
                        let a1071 = v0;
                        let a1072 = 8u32;
                        fixed_u32::bit_and(a1071, a1072)
                    };
                    let a1074 = 0u32;
                    if equal(a1073, a1074) {
                        let a1077 = {
                            let a1075 = v0;
                            let a1076 = 4u32;
                            fixed_u32::bit_and(a1075, a1076)
                        };
                        let a1078 = 0u32;
                        if equal(a1077, a1078) {
                            let a1081 = {
                                let a1079 = v0;
                                let a1080 = 2u32;
                                fixed_u32::bit_and(a1079, a1080)
                            };
                            let a1082 = 0u32;
                            if equal(a1081, a1082) {
                                let a1085 = {
                                    let a1083 = v0;
                                    let a1084 = 1u32;
                                    fixed_u32::bit_and(a1083, a1084)
                                };
                                let a1086 = 0u32;
                                if equal(a1085, a1086) {
                                    Bytes::lit(&[0xa0])
                                } else {
                                    Bytes::lit(&[0xa1])
                                }
                            } else {
                                let a1089 = {
                                    let a1087 = v0;
                                    let a1088 = 1u32;
                                    fixed_u32::bit_and(a1087, a1088)
                                };
                                let a1090 = 0u32;
                                if equal(a1089, a1090) {
                                    Bytes::lit(&[0xa2])
                                } else {
                                    Bytes::lit(&[0xa3])
                                }
                            }
                        } else {
                            let a1093 = {
                                let a1091 = v0;
                                let a1092 = 2u32;
                                fixed_u32::bit_and(a1091, a1092)
                            };
                            let a1094 = 0u32;
                            if equal(a1093, a1094) {
                                let a1097 = {
                                    let a1095 = v0;
                                    let a1096 = 1u32;
                                    fixed_u32::bit_and(a1095, a1096)
                                };
                                let a1098 = 0u32;
                                if equal(a1097, a1098) {
                                    Bytes::lit(&[0xa4])
                                } else {
                                    Bytes::lit(&[0xa5])
                                }
                            } else {
                                let a1101 = {
                                    let a1099 = v0;
                                    let a1100 = 1u32;
                                    fixed_u32::bit_and(a1099, a1100)
                                };
                                let a1102 = 0u32;
                                if equal(a1101, a1102) {
                                    Bytes::lit(&[0xa6])
                                } else {
                                    Bytes::lit(&[0xa7])
                                }
                            }
                        }
                    } else {
                        let a1105 = {
                            let a1103 = v0;
                            let a1104 = 4u32;
                            fixed_u32::bit_and(a1103, a1104)
                        };
                        let a1106 = 0u32;
                        if equal(a1105, a1106) {
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
                                    Bytes::lit(&[0xa8])
                                } else {
                                    Bytes::lit(&[0xa9])
                                }
                            } else {
                                let a1117 = {
                                    let a1115 = v0;
                                    let a1116 = 1u32;
                                    fixed_u32::bit_and(a1115, a1116)
                                };
                                let a1118 = 0u32;
                                if equal(a1117, a1118) {
                                    Bytes::lit(&[0xaa])
                                } else {
                                    Bytes::lit(&[0xab])
                                }
                            }
                        } else {
                            let a1121 = {
                                let a1119 = v0;
                                let a1120 = 2u32;
                                fixed_u32::bit_and(a1119, a1120)
                            };
                            let a1122 = 0u32;
                            if equal(a1121, a1122) {
                                let a1125 = {
                                    let a1123 = v0;
                                    let a1124 = 1u32;
                                    fixed_u32::bit_and(a1123, a1124)
                                };
                                let a1126 = 0u32;
                                if equal(a1125, a1126) {
                                    Bytes::lit(&[0xac])
                                } else {
                                    Bytes::lit(&[0xad])
                                }
                            } else {
                                let a1129 = {
                                    let a1127 = v0;
                                    let a1128 = 1u32;
                                    fixed_u32::bit_and(a1127, a1128)
                                };
                                let a1130 = 0u32;
                                if equal(a1129, a1130) {
                                    Bytes::lit(&[0xae])
                                } else {
                                    Bytes::lit(&[0xaf])
                                }
                            }
                        }
                    }
                } else {
                    let a1133 = {
                        let a1131 = v0;
                        let a1132 = 8u32;
                        fixed_u32::bit_and(a1131, a1132)
                    };
                    let a1134 = 0u32;
                    if equal(a1133, a1134) {
                        let a1137 = {
                            let a1135 = v0;
                            let a1136 = 4u32;
                            fixed_u32::bit_and(a1135, a1136)
                        };
                        let a1138 = 0u32;
                        if equal(a1137, a1138) {
                            let a1141 = {
                                let a1139 = v0;
                                let a1140 = 2u32;
                                fixed_u32::bit_and(a1139, a1140)
                            };
                            let a1142 = 0u32;
                            if equal(a1141, a1142) {
                                let a1145 = {
                                    let a1143 = v0;
                                    let a1144 = 1u32;
                                    fixed_u32::bit_and(a1143, a1144)
                                };
                                let a1146 = 0u32;
                                if equal(a1145, a1146) {
                                    Bytes::lit(&[0xb0])
                                } else {
                                    Bytes::lit(&[0xb1])
                                }
                            } else {
                                let a1149 = {
                                    let a1147 = v0;
                                    let a1148 = 1u32;
                                    fixed_u32::bit_and(a1147, a1148)
                                };
                                let a1150 = 0u32;
                                if equal(a1149, a1150) {
                                    Bytes::lit(&[0xb2])
                                } else {
                                    Bytes::lit(&[0xb3])
                                }
                            }
                        } else {
                            let a1153 = {
                                let a1151 = v0;
                                let a1152 = 2u32;
                                fixed_u32::bit_and(a1151, a1152)
                            };
                            let a1154 = 0u32;
                            if equal(a1153, a1154) {
                                let a1157 = {
                                    let a1155 = v0;
                                    let a1156 = 1u32;
                                    fixed_u32::bit_and(a1155, a1156)
                                };
                                let a1158 = 0u32;
                                if equal(a1157, a1158) {
                                    Bytes::lit(&[0xb4])
                                } else {
                                    Bytes::lit(&[0xb5])
                                }
                            } else {
                                let a1161 = {
                                    let a1159 = v0;
                                    let a1160 = 1u32;
                                    fixed_u32::bit_and(a1159, a1160)
                                };
                                let a1162 = 0u32;
                                if equal(a1161, a1162) {
                                    Bytes::lit(&[0xb6])
                                } else {
                                    Bytes::lit(&[0xb7])
                                }
                            }
                        }
                    } else {
                        let a1165 = {
                            let a1163 = v0;
                            let a1164 = 4u32;
                            fixed_u32::bit_and(a1163, a1164)
                        };
                        let a1166 = 0u32;
                        if equal(a1165, a1166) {
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
                                    Bytes::lit(&[0xb8])
                                } else {
                                    Bytes::lit(&[0xb9])
                                }
                            } else {
                                let a1177 = {
                                    let a1175 = v0;
                                    let a1176 = 1u32;
                                    fixed_u32::bit_and(a1175, a1176)
                                };
                                let a1178 = 0u32;
                                if equal(a1177, a1178) {
                                    Bytes::lit(&[0xba])
                                } else {
                                    Bytes::lit(&[0xbb])
                                }
                            }
                        } else {
                            let a1181 = {
                                let a1179 = v0;
                                let a1180 = 2u32;
                                fixed_u32::bit_and(a1179, a1180)
                            };
                            let a1182 = 0u32;
                            if equal(a1181, a1182) {
                                let a1185 = {
                                    let a1183 = v0;
                                    let a1184 = 1u32;
                                    fixed_u32::bit_and(a1183, a1184)
                                };
                                let a1186 = 0u32;
                                if equal(a1185, a1186) {
                                    Bytes::lit(&[0xbc])
                                } else {
                                    Bytes::lit(&[0xbd])
                                }
                            } else {
                                let a1189 = {
                                    let a1187 = v0;
                                    let a1188 = 1u32;
                                    fixed_u32::bit_and(a1187, a1188)
                                };
                                let a1190 = 0u32;
                                if equal(a1189, a1190) {
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
            let a1193 = {
                let a1191 = v0;
                let a1192 = 32u32;
                fixed_u32::bit_and(a1191, a1192)
            };
            let a1194 = 0u32;
            if equal(a1193, a1194) {
                let a1197 = {
                    let a1195 = v0;
                    let a1196 = 16u32;
                    fixed_u32::bit_and(a1195, a1196)
                };
                let a1198 = 0u32;
                if equal(a1197, a1198) {
                    let a1201 = {
                        let a1199 = v0;
                        let a1200 = 8u32;
                        fixed_u32::bit_and(a1199, a1200)
                    };
                    let a1202 = 0u32;
                    if equal(a1201, a1202) {
                        let a1205 = {
                            let a1203 = v0;
                            let a1204 = 4u32;
                            fixed_u32::bit_and(a1203, a1204)
                        };
                        let a1206 = 0u32;
                        if equal(a1205, a1206) {
                            let a1209 = {
                                let a1207 = v0;
                                let a1208 = 2u32;
                                fixed_u32::bit_and(a1207, a1208)
                            };
                            let a1210 = 0u32;
                            if equal(a1209, a1210) {
                                let a1213 = {
                                    let a1211 = v0;
                                    let a1212 = 1u32;
                                    fixed_u32::bit_and(a1211, a1212)
                                };
                                let a1214 = 0u32;
                                if equal(a1213, a1214) {
                                    Bytes::lit(&[0xc0])
                                } else {
                                    Bytes::lit(&[0xc1])
                                }
                            } else {
                                let a1217 = {
                                    let a1215 = v0;
                                    let a1216 = 1u32;
                                    fixed_u32::bit_and(a1215, a1216)
                                };
                                let a1218 = 0u32;
                                if equal(a1217, a1218) {
                                    Bytes::lit(&[0xc2])
                                } else {
                                    Bytes::lit(&[0xc3])
                                }
                            }
                        } else {
                            let a1221 = {
                                let a1219 = v0;
                                let a1220 = 2u32;
                                fixed_u32::bit_and(a1219, a1220)
                            };
                            let a1222 = 0u32;
                            if equal(a1221, a1222) {
                                let a1225 = {
                                    let a1223 = v0;
                                    let a1224 = 1u32;
                                    fixed_u32::bit_and(a1223, a1224)
                                };
                                let a1226 = 0u32;
                                if equal(a1225, a1226) {
                                    Bytes::lit(&[0xc4])
                                } else {
                                    Bytes::lit(&[0xc5])
                                }
                            } else {
                                let a1229 = {
                                    let a1227 = v0;
                                    let a1228 = 1u32;
                                    fixed_u32::bit_and(a1227, a1228)
                                };
                                let a1230 = 0u32;
                                if equal(a1229, a1230) {
                                    Bytes::lit(&[0xc6])
                                } else {
                                    Bytes::lit(&[0xc7])
                                }
                            }
                        }
                    } else {
                        let a1233 = {
                            let a1231 = v0;
                            let a1232 = 4u32;
                            fixed_u32::bit_and(a1231, a1232)
                        };
                        let a1234 = 0u32;
                        if equal(a1233, a1234) {
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
                                    Bytes::lit(&[0xc8])
                                } else {
                                    Bytes::lit(&[0xc9])
                                }
                            } else {
                                let a1245 = {
                                    let a1243 = v0;
                                    let a1244 = 1u32;
                                    fixed_u32::bit_and(a1243, a1244)
                                };
                                let a1246 = 0u32;
                                if equal(a1245, a1246) {
                                    Bytes::lit(&[0xca])
                                } else {
                                    Bytes::lit(&[0xcb])
                                }
                            }
                        } else {
                            let a1249 = {
                                let a1247 = v0;
                                let a1248 = 2u32;
                                fixed_u32::bit_and(a1247, a1248)
                            };
                            let a1250 = 0u32;
                            if equal(a1249, a1250) {
                                let a1253 = {
                                    let a1251 = v0;
                                    let a1252 = 1u32;
                                    fixed_u32::bit_and(a1251, a1252)
                                };
                                let a1254 = 0u32;
                                if equal(a1253, a1254) {
                                    Bytes::lit(&[0xcc])
                                } else {
                                    Bytes::lit(&[0xcd])
                                }
                            } else {
                                let a1257 = {
                                    let a1255 = v0;
                                    let a1256 = 1u32;
                                    fixed_u32::bit_and(a1255, a1256)
                                };
                                let a1258 = 0u32;
                                if equal(a1257, a1258) {
                                    Bytes::lit(&[0xce])
                                } else {
                                    Bytes::lit(&[0xcf])
                                }
                            }
                        }
                    }
                } else {
                    let a1261 = {
                        let a1259 = v0;
                        let a1260 = 8u32;
                        fixed_u32::bit_and(a1259, a1260)
                    };
                    let a1262 = 0u32;
                    if equal(a1261, a1262) {
                        let a1265 = {
                            let a1263 = v0;
                            let a1264 = 4u32;
                            fixed_u32::bit_and(a1263, a1264)
                        };
                        let a1266 = 0u32;
                        if equal(a1265, a1266) {
                            let a1269 = {
                                let a1267 = v0;
                                let a1268 = 2u32;
                                fixed_u32::bit_and(a1267, a1268)
                            };
                            let a1270 = 0u32;
                            if equal(a1269, a1270) {
                                let a1273 = {
                                    let a1271 = v0;
                                    let a1272 = 1u32;
                                    fixed_u32::bit_and(a1271, a1272)
                                };
                                let a1274 = 0u32;
                                if equal(a1273, a1274) {
                                    Bytes::lit(&[0xd0])
                                } else {
                                    Bytes::lit(&[0xd1])
                                }
                            } else {
                                let a1277 = {
                                    let a1275 = v0;
                                    let a1276 = 1u32;
                                    fixed_u32::bit_and(a1275, a1276)
                                };
                                let a1278 = 0u32;
                                if equal(a1277, a1278) {
                                    Bytes::lit(&[0xd2])
                                } else {
                                    Bytes::lit(&[0xd3])
                                }
                            }
                        } else {
                            let a1281 = {
                                let a1279 = v0;
                                let a1280 = 2u32;
                                fixed_u32::bit_and(a1279, a1280)
                            };
                            let a1282 = 0u32;
                            if equal(a1281, a1282) {
                                let a1285 = {
                                    let a1283 = v0;
                                    let a1284 = 1u32;
                                    fixed_u32::bit_and(a1283, a1284)
                                };
                                let a1286 = 0u32;
                                if equal(a1285, a1286) {
                                    Bytes::lit(&[0xd4])
                                } else {
                                    Bytes::lit(&[0xd5])
                                }
                            } else {
                                let a1289 = {
                                    let a1287 = v0;
                                    let a1288 = 1u32;
                                    fixed_u32::bit_and(a1287, a1288)
                                };
                                let a1290 = 0u32;
                                if equal(a1289, a1290) {
                                    Bytes::lit(&[0xd6])
                                } else {
                                    Bytes::lit(&[0xd7])
                                }
                            }
                        }
                    } else {
                        let a1293 = {
                            let a1291 = v0;
                            let a1292 = 4u32;
                            fixed_u32::bit_and(a1291, a1292)
                        };
                        let a1294 = 0u32;
                        if equal(a1293, a1294) {
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
                                    Bytes::lit(&[0xd8])
                                } else {
                                    Bytes::lit(&[0xd9])
                                }
                            } else {
                                let a1305 = {
                                    let a1303 = v0;
                                    let a1304 = 1u32;
                                    fixed_u32::bit_and(a1303, a1304)
                                };
                                let a1306 = 0u32;
                                if equal(a1305, a1306) {
                                    Bytes::lit(&[0xda])
                                } else {
                                    Bytes::lit(&[0xdb])
                                }
                            }
                        } else {
                            let a1309 = {
                                let a1307 = v0;
                                let a1308 = 2u32;
                                fixed_u32::bit_and(a1307, a1308)
                            };
                            let a1310 = 0u32;
                            if equal(a1309, a1310) {
                                let a1313 = {
                                    let a1311 = v0;
                                    let a1312 = 1u32;
                                    fixed_u32::bit_and(a1311, a1312)
                                };
                                let a1314 = 0u32;
                                if equal(a1313, a1314) {
                                    Bytes::lit(&[0xdc])
                                } else {
                                    Bytes::lit(&[0xdd])
                                }
                            } else {
                                let a1317 = {
                                    let a1315 = v0;
                                    let a1316 = 1u32;
                                    fixed_u32::bit_and(a1315, a1316)
                                };
                                let a1318 = 0u32;
                                if equal(a1317, a1318) {
                                    Bytes::lit(&[0xde])
                                } else {
                                    Bytes::lit(&[0xdf])
                                }
                            }
                        }
                    }
                }
            } else {
                let a1321 = {
                    let a1319 = v0;
                    let a1320 = 16u32;
                    fixed_u32::bit_and(a1319, a1320)
                };
                let a1322 = 0u32;
                if equal(a1321, a1322) {
                    let a1325 = {
                        let a1323 = v0;
                        let a1324 = 8u32;
                        fixed_u32::bit_and(a1323, a1324)
                    };
                    let a1326 = 0u32;
                    if equal(a1325, a1326) {
                        let a1329 = {
                            let a1327 = v0;
                            let a1328 = 4u32;
                            fixed_u32::bit_and(a1327, a1328)
                        };
                        let a1330 = 0u32;
                        if equal(a1329, a1330) {
                            let a1333 = {
                                let a1331 = v0;
                                let a1332 = 2u32;
                                fixed_u32::bit_and(a1331, a1332)
                            };
                            let a1334 = 0u32;
                            if equal(a1333, a1334) {
                                let a1337 = {
                                    let a1335 = v0;
                                    let a1336 = 1u32;
                                    fixed_u32::bit_and(a1335, a1336)
                                };
                                let a1338 = 0u32;
                                if equal(a1337, a1338) {
                                    Bytes::lit(&[0xe0])
                                } else {
                                    Bytes::lit(&[0xe1])
                                }
                            } else {
                                let a1341 = {
                                    let a1339 = v0;
                                    let a1340 = 1u32;
                                    fixed_u32::bit_and(a1339, a1340)
                                };
                                let a1342 = 0u32;
                                if equal(a1341, a1342) {
                                    Bytes::lit(&[0xe2])
                                } else {
                                    Bytes::lit(&[0xe3])
                                }
                            }
                        } else {
                            let a1345 = {
                                let a1343 = v0;
                                let a1344 = 2u32;
                                fixed_u32::bit_and(a1343, a1344)
                            };
                            let a1346 = 0u32;
                            if equal(a1345, a1346) {
                                let a1349 = {
                                    let a1347 = v0;
                                    let a1348 = 1u32;
                                    fixed_u32::bit_and(a1347, a1348)
                                };
                                let a1350 = 0u32;
                                if equal(a1349, a1350) {
                                    Bytes::lit(&[0xe4])
                                } else {
                                    Bytes::lit(&[0xe5])
                                }
                            } else {
                                let a1353 = {
                                    let a1351 = v0;
                                    let a1352 = 1u32;
                                    fixed_u32::bit_and(a1351, a1352)
                                };
                                let a1354 = 0u32;
                                if equal(a1353, a1354) {
                                    Bytes::lit(&[0xe6])
                                } else {
                                    Bytes::lit(&[0xe7])
                                }
                            }
                        }
                    } else {
                        let a1357 = {
                            let a1355 = v0;
                            let a1356 = 4u32;
                            fixed_u32::bit_and(a1355, a1356)
                        };
                        let a1358 = 0u32;
                        if equal(a1357, a1358) {
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
                                    Bytes::lit(&[0xe8])
                                } else {
                                    Bytes::lit(&[0xe9])
                                }
                            } else {
                                let a1369 = {
                                    let a1367 = v0;
                                    let a1368 = 1u32;
                                    fixed_u32::bit_and(a1367, a1368)
                                };
                                let a1370 = 0u32;
                                if equal(a1369, a1370) {
                                    Bytes::lit(&[0xea])
                                } else {
                                    Bytes::lit(&[0xeb])
                                }
                            }
                        } else {
                            let a1373 = {
                                let a1371 = v0;
                                let a1372 = 2u32;
                                fixed_u32::bit_and(a1371, a1372)
                            };
                            let a1374 = 0u32;
                            if equal(a1373, a1374) {
                                let a1377 = {
                                    let a1375 = v0;
                                    let a1376 = 1u32;
                                    fixed_u32::bit_and(a1375, a1376)
                                };
                                let a1378 = 0u32;
                                if equal(a1377, a1378) {
                                    Bytes::lit(&[0xec])
                                } else {
                                    Bytes::lit(&[0xed])
                                }
                            } else {
                                let a1381 = {
                                    let a1379 = v0;
                                    let a1380 = 1u32;
                                    fixed_u32::bit_and(a1379, a1380)
                                };
                                let a1382 = 0u32;
                                if equal(a1381, a1382) {
                                    Bytes::lit(&[0xee])
                                } else {
                                    Bytes::lit(&[0xef])
                                }
                            }
                        }
                    }
                } else {
                    let a1385 = {
                        let a1383 = v0;
                        let a1384 = 8u32;
                        fixed_u32::bit_and(a1383, a1384)
                    };
                    let a1386 = 0u32;
                    if equal(a1385, a1386) {
                        let a1389 = {
                            let a1387 = v0;
                            let a1388 = 4u32;
                            fixed_u32::bit_and(a1387, a1388)
                        };
                        let a1390 = 0u32;
                        if equal(a1389, a1390) {
                            let a1393 = {
                                let a1391 = v0;
                                let a1392 = 2u32;
                                fixed_u32::bit_and(a1391, a1392)
                            };
                            let a1394 = 0u32;
                            if equal(a1393, a1394) {
                                let a1397 = {
                                    let a1395 = v0;
                                    let a1396 = 1u32;
                                    fixed_u32::bit_and(a1395, a1396)
                                };
                                let a1398 = 0u32;
                                if equal(a1397, a1398) {
                                    Bytes::lit(&[0xf0])
                                } else {
                                    Bytes::lit(&[0xf1])
                                }
                            } else {
                                let a1401 = {
                                    let a1399 = v0;
                                    let a1400 = 1u32;
                                    fixed_u32::bit_and(a1399, a1400)
                                };
                                let a1402 = 0u32;
                                if equal(a1401, a1402) {
                                    Bytes::lit(&[0xf2])
                                } else {
                                    Bytes::lit(&[0xf3])
                                }
                            }
                        } else {
                            let a1405 = {
                                let a1403 = v0;
                                let a1404 = 2u32;
                                fixed_u32::bit_and(a1403, a1404)
                            };
                            let a1406 = 0u32;
                            if equal(a1405, a1406) {
                                let a1409 = {
                                    let a1407 = v0;
                                    let a1408 = 1u32;
                                    fixed_u32::bit_and(a1407, a1408)
                                };
                                let a1410 = 0u32;
                                if equal(a1409, a1410) {
                                    Bytes::lit(&[0xf4])
                                } else {
                                    Bytes::lit(&[0xf5])
                                }
                            } else {
                                let a1413 = {
                                    let a1411 = v0;
                                    let a1412 = 1u32;
                                    fixed_u32::bit_and(a1411, a1412)
                                };
                                let a1414 = 0u32;
                                if equal(a1413, a1414) {
                                    Bytes::lit(&[0xf6])
                                } else {
                                    Bytes::lit(&[0xf7])
                                }
                            }
                        }
                    } else {
                        let a1417 = {
                            let a1415 = v0;
                            let a1416 = 4u32;
                            fixed_u32::bit_and(a1415, a1416)
                        };
                        let a1418 = 0u32;
                        if equal(a1417, a1418) {
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
                                    Bytes::lit(&[0xf8])
                                } else {
                                    Bytes::lit(&[0xf9])
                                }
                            } else {
                                let a1429 = {
                                    let a1427 = v0;
                                    let a1428 = 1u32;
                                    fixed_u32::bit_and(a1427, a1428)
                                };
                                let a1430 = 0u32;
                                if equal(a1429, a1430) {
                                    Bytes::lit(&[0xfa])
                                } else {
                                    Bytes::lit(&[0xfb])
                                }
                            }
                        } else {
                            let a1433 = {
                                let a1431 = v0;
                                let a1432 = 2u32;
                                fixed_u32::bit_and(a1431, a1432)
                            };
                            let a1434 = 0u32;
                            if equal(a1433, a1434) {
                                let a1437 = {
                                    let a1435 = v0;
                                    let a1436 = 1u32;
                                    fixed_u32::bit_and(a1435, a1436)
                                };
                                let a1438 = 0u32;
                                if equal(a1437, a1438) {
                                    Bytes::lit(&[0xfc])
                                } else {
                                    Bytes::lit(&[0xfd])
                                }
                            } else {
                                let a1441 = {
                                    let a1439 = v0;
                                    let a1440 = 1u32;
                                    fixed_u32::bit_and(a1439, a1440)
                                };
                                let a1442 = 0u32;
                                if equal(a1441, a1442) {
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

pub fn f57(v0: u32, v1: u32) -> u32 {
    let m1445 = {
        let a1443 = v0;
        let a1444 = v1;
        fixed_u32::shift_right(a1443, a1444)
    };
    match m1445 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f58(v0: Fn10, p1449: (Option<Bytes>, List<Bytes>), p1450: List<Bytes>) -> (Option<Bytes>, List<Bytes>) {
    let m1448 = (p1449, p1450);
    {
        let mut m1448 = m1448;
        loop {
            let step = {
                let (v1, v2) = m1448;
                let m1446 = v2;
                match m1446.uncons() {
                    None => {
                        Err::<((Option<Bytes>, List<Bytes>), List<Bytes>), (Option<Bytes>, List<Bytes>)>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<((Option<Bytes>, List<Bytes>), List<Bytes>), (Option<Bytes>, List<Bytes>)>(({
                            let c1447: Fn10 = v0.clone();
                            c1447.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m1448 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f59(v0: (Option<Bytes>, List<Bytes>), v1: Bytes) -> (Option<Bytes>, List<Bytes>) {
    let m1452 = {
        let (h1451, _) = v0;
        h1451
    };
    match m1452 {
        None => {
            (Some(v1), {
                let (_, h1456) = v0;
                h1456
            })
        }
        Some(v2) => {
            (None::<Bytes>, List::cons({
                let a1453 = v2;
                let a1454 = v1;
                append_bytes(a1453, a1454)
            }, {
                let (_, h1455) = v0;
                h1455
            }))
        }
    }
}

pub fn f60(v0: List<Bytes>, v1: Bytes) -> List<Bytes> {
    List::cons(v1, v0)
}

pub fn f61(v0: Bytes, v1: u64, v2: u64) -> R<u32> {
    Ok(f67(f68(v0.clone(), v1, v2), f68(v0.clone(), v1, {
        let a1457 = v2;
        let a1458 = 1u64;
        nat_add(a1457, a1458)?
    }), f68(v0.clone(), v1, {
        let a1459 = v2;
        let a1460 = 2u64;
        nat_add(a1459, a1460)?
    }), f68(v0, v1, {
        let a1461 = v2;
        let a1462 = 3u64;
        nat_add(a1461, a1462)?
    })))
}

pub fn f62(v0: Fn11, p1466: u64, p1467: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let m1465 = (p1466, p1467);
    {
        let mut m1465 = m1465;
        loop {
            let step = {
                let (v1, v2) = m1465;
                let m1463 = v1;
                if m1463 == 0 {
                    Err::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>(v2)
                } else {
                    let v3 = m1463 - 1;
                    Ok::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>((v3, {
                        let c1464: Fn11 = v0.clone();
                        c1464.apply(v2)
                    }))
                }
            };
            match step {
                Ok(next) => { m1465 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f63(v0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let v1: u64 = {
        let (h1468, _) = v0;
        h1468
    };
    let v2: u32 = {
        let (h1469, _) = {
            let (_, h1470) = v0;
            h1470
        };
        h1469
    };
    let v3: u32 = {
        let (_, h1471) = {
            let (_, h1472) = v0;
            h1472
        };
        h1471
    };
    ({
        let a1473 = v1;
        let a1474 = 2u64;
        let a1475 = 0u64;
        nat_quot(a1473, a1474, a1475)
    }, (f69(v2, 1u32), {
        let a1479 = {
            let a1476 = v1;
            let a1477 = 2u64;
            let a1478 = 0u64;
            nat_rem(a1476, a1477, a1478)
        };
        let a1480 = 1u64;
        if nat_eq(a1479, a1480) {
            let a1481 = v3;
            let a1482 = v2;
            fixed_u32::bit_or(a1481, a1482)
        } else {
            v3
        }
    }))
}

pub fn f64(v0: Adt9) -> Adt4 {
    f70(Fn12::F71, Adt4::C0(0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32), match v0 {
        Adt9::C0(_, _, _, _, _, _, h1483) => {
            h1483
        }
    })
}

pub fn f65(v0: Adt1, v1: Adt4, v2: u32, v3: u32, v4: u32, v5: u32) -> R<Adt9> {
    let v6: (u64, Adt12) = f72(Fn13::F73(v0.clone(), v1.clone()), 800u64, (0u64, Adt12::C0(f74(v0.clone(), v2, v3, v4, v5), Adt4::C0(0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32), List::<Adt10>::nil())))?;
    Ok(Adt9::C0(v0, v1, v2, v3, v4, v5, {
        let m1486 = {
            let (_, h1485) = v6;
            h1485
        };
        match m1486 {
            Adt12::C0(_, _, h1484) => {
                h1484
            }
        }
    }))
}

pub fn f66(v0: Adt1, v1: u32, v2: Adt8) -> R<Option<Adt8>> {
    let a1491 = {
        let a1488 = match v2.clone() {
            Adt8::C0(_, h1487, _, _) => {
                h1487
            }
        };
        let a1489 = 2u64;
        let a1490 = 0u64;
        nat_rem(a1488, a1489, a1490)
    };
    let a1492 = 0u64;
    if nat_eq(a1491, a1492) {
        let m1494 = match v2.clone() {
            Adt8::C0(_, _, h1493, _) => {
                h1493
            }
        };
        match m1494.uncons() {
            None => {
                Ok(None::<Adt8>)
            }
            Some((v3, v4)) => {
                let v5: Adt3 = f32(f33(v3.clone(), match v2.clone() {
                    Adt8::C0(h1495, _, _, _) => {
                        h1495
                    }
                }, v0, v1)?)?;
                Ok(Some(Adt8::C0(Adt5::C0(f20(match v5.clone() {
                    Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h1496) => {
                        h1496
                    }
                }), match v3.clone() {
                    Adt5::C0(_, h1497, _) => {
                        h1497
                    }
                }, {
                    let a1502 = match v3.clone() {
                        Adt5::C0(_, _, h1498) => {
                            h1498
                        }
                    };
                    let a1503 = {
                        let m1501 = match v2.clone() {
                            Adt8::C0(h1500, _, _, _) => {
                                h1500
                            }
                        };
                        match m1501 {
                            Adt5::C0(_, _, h1499) => {
                                h1499
                            }
                        }
                    };
                    nat_add(a1502, a1503)?
                }), {
                    let a1505 = match v2.clone() {
                        Adt8::C0(_, h1504, _, _) => {
                            h1504
                        }
                    };
                    let a1506 = 2u64;
                    let a1507 = 0u64;
                    nat_quot(a1505, a1506, a1507)
                }, v4, List::cons(Adt2::C0(v5), List::cons(Adt2::C2(v3, {
                    let a1509 = match v2.clone() {
                        Adt8::C0(_, _, h1508, _) => {
                            h1508
                        }
                    };
                    length_list(a1509)
                }), match v2 {
                    Adt8::C0(_, _, _, h1510) => {
                        h1510
                    }
                })))))
            }
        }
    } else {
        Ok(None::<Adt8>)
    }
}

pub fn f67(v0: u32, v1: u32, v2: u32, v3: u32) -> u32 {
    let a1515 = v0;
    let a1516 = {
        let a1513 = f69(v1, 8u32);
        let a1514 = {
            let a1511 = f69(v2, 16u32);
            let a1512 = f69(v3, 24u32);
            fixed_u32::bit_or(a1511, a1512)
        };
        fixed_u32::bit_or(a1513, a1514)
    };
    fixed_u32::bit_or(a1515, a1516)
}

pub fn f68(v0: Bytes, v1: u64, v2: u64) -> u32 {
    let a1517 = v2;
    let a1518 = v1;
    if nat_lt(a1517, a1518) {
        let m1521 = {
            let a1519 = v0;
            let a1520 = v2;
            index_bytes(a1519, a1520)
        };
        match m1521 {
            None => {
                0u32
            }
            Some(v3) => {
                let m1523 = {
                    let a1522 = v3;
                    fixed_u32::convert(i128::from(a1522))
                };
                match m1523 {
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

pub fn f69(v0: u32, v1: u32) -> u32 {
    let m1526 = {
        let a1524 = v0;
        let a1525 = v1;
        fixed_u32::shift_left(a1524, a1525)
    };
    match m1526 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f70(v0: Fn12, p1530: Adt4, p1531: List<Adt10>) -> Adt4 {
    let m1529 = (p1530, p1531);
    {
        let mut m1529 = m1529;
        loop {
            let step = {
                let (v1, v2) = m1529;
                let m1527 = v2;
                match m1527.uncons() {
                    None => {
                        Err::<(Adt4, List<Adt10>), Adt4>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt4, List<Adt10>), Adt4>(({
                            let c1528: Fn12 = v0.clone();
                            c1528.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m1529 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f71(v0: Adt4, v1: Adt10) -> Adt4 {
    let a1533 = match v1.clone() {
        Adt10::C0(h1532, _, _, _, _, _, _, _, _) => {
            h1532
        }
    };
    let a1534 = 7u64;
    if nat_eq(a1533, a1534) {
        f75(v0, match v1.clone() {
            Adt10::C0(_, _, _, _, h1535, _, _, _, _) => {
                h1535
            }
        }, match v1 {
            Adt10::C0(_, _, _, _, _, _, _, _, h1536) => {
                h1536
            }
        })
    } else {
        v0
    }
}

pub fn f72(v0: Fn13, p1540: u64, p1541: (u64, Adt12)) -> R<(u64, Adt12)> {
    let m1539 = (p1540, p1541);
    {
        let mut m1539 = m1539;
        loop {
            let step = {
                let (v1, v2) = m1539;
                let m1537 = v1;
                if m1537 == 0 {
                    Err::<(u64, (u64, Adt12)), R<(u64, Adt12)>>(Ok(v2))
                } else {
                    let v3 = m1537 - 1;
                    Ok::<(u64, (u64, Adt12)), R<(u64, Adt12)>>((v3, {
                        let c1538: Fn13 = v0.clone();
                        c1538.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m1539 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f73(v0: Adt1, v1: Adt4, v2: (u64, Adt12)) -> R<(u64, Adt12)> {
    Ok(({
        let a1543 = {
            let (h1542, _) = v2.clone();
            h1542
        };
        let a1544 = 1u64;
        nat_add(a1543, a1544)?
    }, f76(v0, v1, {
        let (_, h1545) = v2;
        h1545
    }, {
        let (h1546, _) = v2;
        h1546
    })?))
}

pub fn f74(v0: Adt1, v1: u32, v2: u32, v3: u32, v4: u32) -> Adt4 {
    Adt4::C0(match v0.clone() {
        Adt1::C0(h1547, _, _, _, _, _, _, _) => {
            h1547
        }
    }, match v0.clone() {
        Adt1::C0(_, h1548, _, _, _, _, _, _) => {
            h1548
        }
    }, match v0.clone() {
        Adt1::C0(_, _, h1549, _, _, _, _, _) => {
            h1549
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, h1550, _, _, _, _) => {
            h1550
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, h1551, _, _, _) => {
            h1551
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, _, h1552, _, _) => {
            h1552
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, _, _, h1553, _) => {
            h1553
        }
    }, match v0 {
        Adt1::C0(_, _, _, _, _, _, _, h1554) => {
            h1554
        }
    }, 1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, v1, v2, v3, v4)
}

pub fn f75(v0: Adt4, v1: u64, v2: u32) -> Adt4 {
    let a1555 = v1;
    let a1556 = 8u64;
    if nat_lt(a1555, a1556) {
        let a1557 = v1;
        let a1558 = 4u64;
        if nat_lt(a1557, a1558) {
            let a1559 = v1;
            let a1560 = 2u64;
            if nat_lt(a1559, a1560) {
                let a1561 = v1;
                let a1562 = 1u64;
                if nat_lt(a1561, a1562) {
                    Adt4::C0(v2, match v0.clone() {
                        Adt4::C0(_, h1563, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1563
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1564, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1564
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1565, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1565
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1566, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1566
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1567, _, _, _, _, _, _, _, _, _, _) => {
                            h1567
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1568, _, _, _, _, _, _, _, _, _) => {
                            h1568
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1569, _, _, _, _, _, _, _, _) => {
                            h1569
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1570, _, _, _, _, _, _, _) => {
                            h1570
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1571, _, _, _, _, _, _) => {
                            h1571
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1572, _, _, _, _, _) => {
                            h1572
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1573, _, _, _, _) => {
                            h1573
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1574, _, _, _) => {
                            h1574
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1575, _, _) => {
                            h1575
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1576, _) => {
                            h1576
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1577) => {
                            h1577
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1578, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1578
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, h1579, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1579
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1580, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1580
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1581, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1581
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1582, _, _, _, _, _, _, _, _, _, _) => {
                            h1582
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1583, _, _, _, _, _, _, _, _, _) => {
                            h1583
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1584, _, _, _, _, _, _, _, _) => {
                            h1584
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1585, _, _, _, _, _, _, _) => {
                            h1585
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1586, _, _, _, _, _, _) => {
                            h1586
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1587, _, _, _, _, _) => {
                            h1587
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1588, _, _, _, _) => {
                            h1588
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1589, _, _, _) => {
                            h1589
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1590, _, _) => {
                            h1590
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1591, _) => {
                            h1591
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1592) => {
                            h1592
                        }
                    })
                }
            } else {
                let a1593 = v1;
                let a1594 = 3u64;
                if nat_lt(a1593, a1594) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1595, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1595
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1596, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1596
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, h1597, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1597
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1598, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1598
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1599, _, _, _, _, _, _, _, _, _, _) => {
                            h1599
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1600, _, _, _, _, _, _, _, _, _) => {
                            h1600
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1601, _, _, _, _, _, _, _, _) => {
                            h1601
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1602, _, _, _, _, _, _, _) => {
                            h1602
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1603, _, _, _, _, _, _) => {
                            h1603
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1604, _, _, _, _, _) => {
                            h1604
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1605, _, _, _, _) => {
                            h1605
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1606, _, _, _) => {
                            h1606
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1607, _, _) => {
                            h1607
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1608, _) => {
                            h1608
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1609) => {
                            h1609
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1610, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1610
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1611, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1611
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1612, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1612
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1613, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1613
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1614, _, _, _, _, _, _, _, _, _, _) => {
                            h1614
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1615, _, _, _, _, _, _, _, _, _) => {
                            h1615
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1616, _, _, _, _, _, _, _, _) => {
                            h1616
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1617, _, _, _, _, _, _, _) => {
                            h1617
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1618, _, _, _, _, _, _) => {
                            h1618
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1619, _, _, _, _, _) => {
                            h1619
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1620, _, _, _, _) => {
                            h1620
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1621, _, _, _) => {
                            h1621
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1622, _, _) => {
                            h1622
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1623, _) => {
                            h1623
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1624) => {
                            h1624
                        }
                    })
                }
            }
        } else {
            let a1625 = v1;
            let a1626 = 6u64;
            if nat_lt(a1625, a1626) {
                let a1627 = v1;
                let a1628 = 5u64;
                if nat_lt(a1627, a1628) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1629, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1629
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1630, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1630
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1631, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1631
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1632, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1632
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1633, _, _, _, _, _, _, _, _, _, _) => {
                            h1633
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1634, _, _, _, _, _, _, _, _, _) => {
                            h1634
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1635, _, _, _, _, _, _, _, _) => {
                            h1635
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1636, _, _, _, _, _, _, _) => {
                            h1636
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1637, _, _, _, _, _, _) => {
                            h1637
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1638, _, _, _, _, _) => {
                            h1638
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1639, _, _, _, _) => {
                            h1639
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1640, _, _, _) => {
                            h1640
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1641, _, _) => {
                            h1641
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1642, _) => {
                            h1642
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1643) => {
                            h1643
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1644, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1644
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1645, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1645
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1646, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1646
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1647, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1647
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1648, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1648
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1649, _, _, _, _, _, _, _, _, _) => {
                            h1649
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1650, _, _, _, _, _, _, _, _) => {
                            h1650
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1651, _, _, _, _, _, _, _) => {
                            h1651
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1652, _, _, _, _, _, _) => {
                            h1652
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1653, _, _, _, _, _) => {
                            h1653
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1654, _, _, _, _) => {
                            h1654
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1655, _, _, _) => {
                            h1655
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1656, _, _) => {
                            h1656
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1657, _) => {
                            h1657
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1658) => {
                            h1658
                        }
                    })
                }
            } else {
                let a1659 = v1;
                let a1660 = 7u64;
                if nat_lt(a1659, a1660) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1661, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1661
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1662, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1662
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1663, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1663
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1664, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1664
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1665, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1665
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1666, _, _, _, _, _, _, _, _, _, _) => {
                            h1666
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1667, _, _, _, _, _, _, _, _) => {
                            h1667
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1668, _, _, _, _, _, _, _) => {
                            h1668
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1669, _, _, _, _, _, _) => {
                            h1669
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1670, _, _, _, _, _) => {
                            h1670
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1671, _, _, _, _) => {
                            h1671
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1672, _, _, _) => {
                            h1672
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1673, _, _) => {
                            h1673
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1674, _) => {
                            h1674
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1675) => {
                            h1675
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1676, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1676
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1677, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1677
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1678, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1678
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1679, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1679
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1680, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1680
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1681, _, _, _, _, _, _, _, _, _, _) => {
                            h1681
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1682, _, _, _, _, _, _, _, _, _) => {
                            h1682
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1683, _, _, _, _, _, _, _) => {
                            h1683
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1684, _, _, _, _, _, _) => {
                            h1684
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1685, _, _, _, _, _) => {
                            h1685
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1686, _, _, _, _) => {
                            h1686
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1687, _, _, _) => {
                            h1687
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1688, _, _) => {
                            h1688
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1689, _) => {
                            h1689
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1690) => {
                            h1690
                        }
                    })
                }
            }
        }
    } else {
        let a1691 = v1;
        let a1692 = 12u64;
        if nat_lt(a1691, a1692) {
            let a1693 = v1;
            let a1694 = 10u64;
            if nat_lt(a1693, a1694) {
                let a1695 = v1;
                let a1696 = 9u64;
                if nat_lt(a1695, a1696) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1697, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1697
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1698, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1698
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1699, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1699
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1700, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1700
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1701, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1701
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1702, _, _, _, _, _, _, _, _, _, _) => {
                            h1702
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1703, _, _, _, _, _, _, _, _, _) => {
                            h1703
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1704, _, _, _, _, _, _, _, _) => {
                            h1704
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1705, _, _, _, _, _, _) => {
                            h1705
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1706, _, _, _, _, _) => {
                            h1706
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1707, _, _, _, _) => {
                            h1707
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1708, _, _, _) => {
                            h1708
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1709, _, _) => {
                            h1709
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1710, _) => {
                            h1710
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1711) => {
                            h1711
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1712, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1712
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1713, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1713
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1714, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1714
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1715, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1715
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1716, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1716
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1717, _, _, _, _, _, _, _, _, _, _) => {
                            h1717
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1718, _, _, _, _, _, _, _, _, _) => {
                            h1718
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1719, _, _, _, _, _, _, _, _) => {
                            h1719
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1720, _, _, _, _, _, _, _) => {
                            h1720
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1721, _, _, _, _, _) => {
                            h1721
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1722, _, _, _, _) => {
                            h1722
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1723, _, _, _) => {
                            h1723
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1724, _, _) => {
                            h1724
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1725, _) => {
                            h1725
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1726) => {
                            h1726
                        }
                    })
                }
            } else {
                let a1727 = v1;
                let a1728 = 11u64;
                if nat_lt(a1727, a1728) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1729, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1729
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1730, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1730
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1731, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1731
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1732, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1732
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1733, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1733
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1734, _, _, _, _, _, _, _, _, _, _) => {
                            h1734
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1735, _, _, _, _, _, _, _, _, _) => {
                            h1735
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1736, _, _, _, _, _, _, _, _) => {
                            h1736
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1737, _, _, _, _, _, _, _) => {
                            h1737
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1738, _, _, _, _, _, _) => {
                            h1738
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1739, _, _, _, _) => {
                            h1739
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1740, _, _, _) => {
                            h1740
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1741, _, _) => {
                            h1741
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1742, _) => {
                            h1742
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1743) => {
                            h1743
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1744, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1744
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1745, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1745
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1746, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1746
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1747, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1747
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1748, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1748
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1749, _, _, _, _, _, _, _, _, _, _) => {
                            h1749
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1750, _, _, _, _, _, _, _, _, _) => {
                            h1750
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1751, _, _, _, _, _, _, _, _) => {
                            h1751
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1752, _, _, _, _, _, _, _) => {
                            h1752
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1753, _, _, _, _, _, _) => {
                            h1753
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1754, _, _, _, _, _) => {
                            h1754
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1755, _, _, _) => {
                            h1755
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1756, _, _) => {
                            h1756
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1757, _) => {
                            h1757
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1758) => {
                            h1758
                        }
                    })
                }
            }
        } else {
            let a1759 = v1;
            let a1760 = 14u64;
            if nat_lt(a1759, a1760) {
                let a1761 = v1;
                let a1762 = 13u64;
                if nat_lt(a1761, a1762) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1763, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1763
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1764, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1764
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1765, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1765
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1766, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1766
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1767, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1767
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1768, _, _, _, _, _, _, _, _, _, _) => {
                            h1768
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1769, _, _, _, _, _, _, _, _, _) => {
                            h1769
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1770, _, _, _, _, _, _, _, _) => {
                            h1770
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1771, _, _, _, _, _, _, _) => {
                            h1771
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1772, _, _, _, _, _, _) => {
                            h1772
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1773, _, _, _, _, _) => {
                            h1773
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1774, _, _, _, _) => {
                            h1774
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1775, _, _) => {
                            h1775
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1776, _) => {
                            h1776
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1777) => {
                            h1777
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1778, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1778
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1779, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1779
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1780, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1780
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1781, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1781
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1782, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1782
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1783, _, _, _, _, _, _, _, _, _, _) => {
                            h1783
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1784, _, _, _, _, _, _, _, _, _) => {
                            h1784
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1785, _, _, _, _, _, _, _, _) => {
                            h1785
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1786, _, _, _, _, _, _, _) => {
                            h1786
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1787, _, _, _, _, _, _) => {
                            h1787
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1788, _, _, _, _, _) => {
                            h1788
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1789, _, _, _, _) => {
                            h1789
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1790, _, _, _) => {
                            h1790
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1791, _) => {
                            h1791
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1792) => {
                            h1792
                        }
                    })
                }
            } else {
                let a1793 = v1;
                let a1794 = 15u64;
                if nat_lt(a1793, a1794) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1795, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1795
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1796, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1796
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1797, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1797
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1798, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1798
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1799, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1799
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1800, _, _, _, _, _, _, _, _, _, _) => {
                            h1800
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1801, _, _, _, _, _, _, _, _, _) => {
                            h1801
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1802, _, _, _, _, _, _, _, _) => {
                            h1802
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1803, _, _, _, _, _, _, _) => {
                            h1803
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1804, _, _, _, _, _, _) => {
                            h1804
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1805, _, _, _, _, _) => {
                            h1805
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1806, _, _, _, _) => {
                            h1806
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1807, _, _, _) => {
                            h1807
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1808, _, _) => {
                            h1808
                        }
                    }, v2, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1809) => {
                            h1809
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1810, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1810
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1811, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1811
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1812, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1812
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1813, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1813
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1814, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1814
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1815, _, _, _, _, _, _, _, _, _, _) => {
                            h1815
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1816, _, _, _, _, _, _, _, _, _) => {
                            h1816
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1817, _, _, _, _, _, _, _, _) => {
                            h1817
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1818, _, _, _, _, _, _, _) => {
                            h1818
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1819, _, _, _, _, _, _) => {
                            h1819
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1820, _, _, _, _, _) => {
                            h1820
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1821, _, _, _, _) => {
                            h1821
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1822, _, _, _) => {
                            h1822
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1823, _, _) => {
                            h1823
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1824, _) => {
                            h1824
                        }
                    }, v2)
                }
            }
        }
    }
}

pub fn f76(v0: Adt1, v1: Adt4, v2: Adt12, v3: u64) -> R<Adt12> {
    let v4: u64 = {
        let a1825 = v3;
        let a1826 = 784u64;
        if nat_lt(a1825, a1826) {
            let a1827 = v3;
            let a1828 = 112u64;
            let a1829 = 0u64;
            nat_quot(a1827, a1828, a1829)
        } else {
            7u64
        }
    };
    let v5: Adt13 = f77({
        let a1830 = v3;
        let a1831 = 784u64;
        if nat_lt(a1830, a1831) {
            let a1832 = v3;
            let a1833 = 112u64;
            let a1834 = 0u64;
            nat_rem(a1832, a1833, a1834)
        } else {
            let a1835 = v3;
            let a1836 = 672u64;
            nat_sub(a1835, a1836)
        }
    });
    let v6: Adt4 = match v2.clone() {
        Adt12::C0(h1837, _, _) => {
            h1837
        }
    };
    let v7: u32 = f78(v6.clone(), match v5.clone() {
        Adt13::C0(_, _, _, h1838, _) => {
            h1838
        }
    });
    let v8: u32 = {
        let m1840 = match v5.clone() {
            Adt13::C0(_, _, h1839, _, _) => {
                h1839
            }
        };
        match m1840 {
            Adt11::C0 => {
                f78(v6.clone(), match v5.clone() {
                    Adt13::C0(_, _, _, _, h1841) => {
                        h1841
                    }
                })
            }
            Adt11::C1 => {
                f78(v1, f79(v4, match v5.clone() {
                    Adt13::C0(_, _, _, _, h1842) => {
                        h1842
                    }
                })?)
            }
            Adt11::C2 => {
                f78(v6.clone(), match v5.clone() {
                    Adt13::C0(_, _, _, _, h1843) => {
                        h1843
                    }
                })
            }
            Adt11::C3 => {
                f80(match v5.clone() {
                    Adt13::C0(_, _, _, _, h1844) => {
                        h1844
                    }
                })
            }
            Adt11::C4 => {
                f81(v0, match v5.clone() {
                    Adt13::C0(_, _, _, _, h1845) => {
                        h1845
                    }
                })
            }
        }
    };
    let v9: u32 = f82(match v5.clone() {
        Adt13::C0(_, _, h1846, _, _) => {
            h1846
        }
    }, v7, v8);
    Ok(Adt12::C0({
        let a1847 = v4;
        let a1848 = 7u64;
        if nat_eq(a1847, a1848) {
            v6
        } else {
            f75(v6, match v5.clone() {
                Adt13::C0(_, _, _, h1849, _) => {
                    h1849
                }
            }, v9)
        }
    }, {
        let a1850 = v4;
        let a1851 = 7u64;
        if nat_eq(a1850, a1851) {
            f75(match v2.clone() {
                Adt12::C0(_, h1852, _) => {
                    h1852
                }
            }, match v5.clone() {
                Adt13::C0(_, _, _, h1853, _) => {
                    h1853
                }
            }, v9)
        } else {
            match v2.clone() {
                Adt12::C0(_, h1854, _) => {
                    h1854
                }
            }
        }
    }, List::cons(Adt10::C0(v4, match v5.clone() {
        Adt13::C0(h1855, _, _, _, _) => {
            h1855
        }
    }, match v5.clone() {
        Adt13::C0(_, h1856, _, _, _) => {
            h1856
        }
    }, match v5.clone() {
        Adt13::C0(_, _, h1857, _, _) => {
            h1857
        }
    }, match v5.clone() {
        Adt13::C0(_, _, _, h1858, _) => {
            h1858
        }
    }, match v5 {
        Adt13::C0(_, _, _, _, h1859) => {
            h1859
        }
    }, v7, v8, v9), match v2 {
        Adt12::C0(_, _, h1860) => {
            h1860
        }
    })))
}

pub fn f77(v0: u64) -> Adt13 {
    let a1861 = v0;
    let a1862 = 64u64;
    if nat_lt(a1861, a1862) {
        let a1863 = v0;
        let a1864 = 32u64;
        if nat_lt(a1863, a1864) {
            let a1865 = v0;
            let a1866 = 16u64;
            if nat_lt(a1865, a1866) {
                let a1867 = v0;
                let a1868 = 8u64;
                if nat_lt(a1867, a1868) {
                    let a1869 = v0;
                    let a1870 = 4u64;
                    if nat_lt(a1869, a1870) {
                        let a1871 = v0;
                        let a1872 = 2u64;
                        if nat_lt(a1871, a1872) {
                            let a1873 = v0;
                            let a1874 = 1u64;
                            if nat_lt(a1873, a1874) {
                                Adt13::C0(0u64, 0u64, Adt11::C0, 0u64, 4u64)
                            } else {
                                Adt13::C0(0u64, 1u64, Adt11::C1, 0u64, 0u64)
                            }
                        } else {
                            let a1875 = v0;
                            let a1876 = 3u64;
                            if nat_lt(a1875, a1876) {
                                Adt13::C0(0u64, 2u64, Adt11::C2, 12u64, 0u64)
                            } else {
                                Adt13::C0(0u64, 3u64, Adt11::C3, 12u64, 16u64)
                            }
                        }
                    } else {
                        let a1877 = v0;
                        let a1878 = 6u64;
                        if nat_lt(a1877, a1878) {
                            let a1879 = v0;
                            let a1880 = 5u64;
                            if nat_lt(a1879, a1880) {
                                Adt13::C0(0u64, 4u64, Adt11::C0, 8u64, 12u64)
                            } else {
                                Adt13::C0(0u64, 5u64, Adt11::C2, 4u64, 8u64)
                            }
                        } else {
                            let a1881 = v0;
                            let a1882 = 7u64;
                            if nat_lt(a1881, a1882) {
                                Adt13::C0(0u64, 6u64, Adt11::C3, 4u64, 12u64)
                            } else {
                                Adt13::C0(0u64, 7u64, Adt11::C0, 0u64, 4u64)
                            }
                        }
                    }
                } else {
                    let a1883 = v0;
                    let a1884 = 12u64;
                    if nat_lt(a1883, a1884) {
                        let a1885 = v0;
                        let a1886 = 10u64;
                        if nat_lt(a1885, a1886) {
                            let a1887 = v0;
                            let a1888 = 9u64;
                            if nat_lt(a1887, a1888) {
                                Adt13::C0(0u64, 8u64, Adt11::C1, 0u64, 1u64)
                            } else {
                                Adt13::C0(0u64, 9u64, Adt11::C2, 12u64, 0u64)
                            }
                        } else {
                            let a1889 = v0;
                            let a1890 = 11u64;
                            if nat_lt(a1889, a1890) {
                                Adt13::C0(0u64, 10u64, Adt11::C3, 12u64, 8u64)
                            } else {
                                Adt13::C0(0u64, 11u64, Adt11::C0, 8u64, 12u64)
                            }
                        }
                    } else {
                        let a1891 = v0;
                        let a1892 = 14u64;
                        if nat_lt(a1891, a1892) {
                            let a1893 = v0;
                            let a1894 = 13u64;
                            if nat_lt(a1893, a1894) {
                                Adt13::C0(0u64, 12u64, Adt11::C2, 4u64, 8u64)
                            } else {
                                Adt13::C0(0u64, 13u64, Adt11::C3, 4u64, 7u64)
                            }
                        } else {
                            let a1895 = v0;
                            let a1896 = 15u64;
                            if nat_lt(a1895, a1896) {
                                Adt13::C0(1u64, 0u64, Adt11::C0, 1u64, 5u64)
                            } else {
                                Adt13::C0(1u64, 1u64, Adt11::C1, 1u64, 2u64)
                            }
                        }
                    }
                }
            } else {
                let a1897 = v0;
                let a1898 = 24u64;
                if nat_lt(a1897, a1898) {
                    let a1899 = v0;
                    let a1900 = 20u64;
                    if nat_lt(a1899, a1900) {
                        let a1901 = v0;
                        let a1902 = 18u64;
                        if nat_lt(a1901, a1902) {
                            let a1903 = v0;
                            let a1904 = 17u64;
                            if nat_lt(a1903, a1904) {
                                Adt13::C0(1u64, 2u64, Adt11::C2, 13u64, 1u64)
                            } else {
                                Adt13::C0(1u64, 3u64, Adt11::C3, 13u64, 16u64)
                            }
                        } else {
                            let a1905 = v0;
                            let a1906 = 19u64;
                            if nat_lt(a1905, a1906) {
                                Adt13::C0(1u64, 4u64, Adt11::C0, 9u64, 13u64)
                            } else {
                                Adt13::C0(1u64, 5u64, Adt11::C2, 5u64, 9u64)
                            }
                        }
                    } else {
                        let a1907 = v0;
                        let a1908 = 22u64;
                        if nat_lt(a1907, a1908) {
                            let a1909 = v0;
                            let a1910 = 21u64;
                            if nat_lt(a1909, a1910) {
                                Adt13::C0(1u64, 6u64, Adt11::C3, 5u64, 12u64)
                            } else {
                                Adt13::C0(1u64, 7u64, Adt11::C0, 1u64, 5u64)
                            }
                        } else {
                            let a1911 = v0;
                            let a1912 = 23u64;
                            if nat_lt(a1911, a1912) {
                                Adt13::C0(1u64, 8u64, Adt11::C1, 1u64, 3u64)
                            } else {
                                Adt13::C0(1u64, 9u64, Adt11::C2, 13u64, 1u64)
                            }
                        }
                    }
                } else {
                    let a1913 = v0;
                    let a1914 = 28u64;
                    if nat_lt(a1913, a1914) {
                        let a1915 = v0;
                        let a1916 = 26u64;
                        if nat_lt(a1915, a1916) {
                            let a1917 = v0;
                            let a1918 = 25u64;
                            if nat_lt(a1917, a1918) {
                                Adt13::C0(1u64, 10u64, Adt11::C3, 13u64, 8u64)
                            } else {
                                Adt13::C0(1u64, 11u64, Adt11::C0, 9u64, 13u64)
                            }
                        } else {
                            let a1919 = v0;
                            let a1920 = 27u64;
                            if nat_lt(a1919, a1920) {
                                Adt13::C0(1u64, 12u64, Adt11::C2, 5u64, 9u64)
                            } else {
                                Adt13::C0(1u64, 13u64, Adt11::C3, 5u64, 7u64)
                            }
                        }
                    } else {
                        let a1921 = v0;
                        let a1922 = 30u64;
                        if nat_lt(a1921, a1922) {
                            let a1923 = v0;
                            let a1924 = 29u64;
                            if nat_lt(a1923, a1924) {
                                Adt13::C0(2u64, 0u64, Adt11::C0, 2u64, 6u64)
                            } else {
                                Adt13::C0(2u64, 1u64, Adt11::C1, 2u64, 4u64)
                            }
                        } else {
                            let a1925 = v0;
                            let a1926 = 31u64;
                            if nat_lt(a1925, a1926) {
                                Adt13::C0(2u64, 2u64, Adt11::C2, 14u64, 2u64)
                            } else {
                                Adt13::C0(2u64, 3u64, Adt11::C3, 14u64, 16u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a1927 = v0;
            let a1928 = 48u64;
            if nat_lt(a1927, a1928) {
                let a1929 = v0;
                let a1930 = 40u64;
                if nat_lt(a1929, a1930) {
                    let a1931 = v0;
                    let a1932 = 36u64;
                    if nat_lt(a1931, a1932) {
                        let a1933 = v0;
                        let a1934 = 34u64;
                        if nat_lt(a1933, a1934) {
                            let a1935 = v0;
                            let a1936 = 33u64;
                            if nat_lt(a1935, a1936) {
                                Adt13::C0(2u64, 4u64, Adt11::C0, 10u64, 14u64)
                            } else {
                                Adt13::C0(2u64, 5u64, Adt11::C2, 6u64, 10u64)
                            }
                        } else {
                            let a1937 = v0;
                            let a1938 = 35u64;
                            if nat_lt(a1937, a1938) {
                                Adt13::C0(2u64, 6u64, Adt11::C3, 6u64, 12u64)
                            } else {
                                Adt13::C0(2u64, 7u64, Adt11::C0, 2u64, 6u64)
                            }
                        }
                    } else {
                        let a1939 = v0;
                        let a1940 = 38u64;
                        if nat_lt(a1939, a1940) {
                            let a1941 = v0;
                            let a1942 = 37u64;
                            if nat_lt(a1941, a1942) {
                                Adt13::C0(2u64, 8u64, Adt11::C1, 2u64, 5u64)
                            } else {
                                Adt13::C0(2u64, 9u64, Adt11::C2, 14u64, 2u64)
                            }
                        } else {
                            let a1943 = v0;
                            let a1944 = 39u64;
                            if nat_lt(a1943, a1944) {
                                Adt13::C0(2u64, 10u64, Adt11::C3, 14u64, 8u64)
                            } else {
                                Adt13::C0(2u64, 11u64, Adt11::C0, 10u64, 14u64)
                            }
                        }
                    }
                } else {
                    let a1945 = v0;
                    let a1946 = 44u64;
                    if nat_lt(a1945, a1946) {
                        let a1947 = v0;
                        let a1948 = 42u64;
                        if nat_lt(a1947, a1948) {
                            let a1949 = v0;
                            let a1950 = 41u64;
                            if nat_lt(a1949, a1950) {
                                Adt13::C0(2u64, 12u64, Adt11::C2, 6u64, 10u64)
                            } else {
                                Adt13::C0(2u64, 13u64, Adt11::C3, 6u64, 7u64)
                            }
                        } else {
                            let a1951 = v0;
                            let a1952 = 43u64;
                            if nat_lt(a1951, a1952) {
                                Adt13::C0(3u64, 0u64, Adt11::C0, 3u64, 7u64)
                            } else {
                                Adt13::C0(3u64, 1u64, Adt11::C1, 3u64, 6u64)
                            }
                        }
                    } else {
                        let a1953 = v0;
                        let a1954 = 46u64;
                        if nat_lt(a1953, a1954) {
                            let a1955 = v0;
                            let a1956 = 45u64;
                            if nat_lt(a1955, a1956) {
                                Adt13::C0(3u64, 2u64, Adt11::C2, 15u64, 3u64)
                            } else {
                                Adt13::C0(3u64, 3u64, Adt11::C3, 15u64, 16u64)
                            }
                        } else {
                            let a1957 = v0;
                            let a1958 = 47u64;
                            if nat_lt(a1957, a1958) {
                                Adt13::C0(3u64, 4u64, Adt11::C0, 11u64, 15u64)
                            } else {
                                Adt13::C0(3u64, 5u64, Adt11::C2, 7u64, 11u64)
                            }
                        }
                    }
                }
            } else {
                let a1959 = v0;
                let a1960 = 56u64;
                if nat_lt(a1959, a1960) {
                    let a1961 = v0;
                    let a1962 = 52u64;
                    if nat_lt(a1961, a1962) {
                        let a1963 = v0;
                        let a1964 = 50u64;
                        if nat_lt(a1963, a1964) {
                            let a1965 = v0;
                            let a1966 = 49u64;
                            if nat_lt(a1965, a1966) {
                                Adt13::C0(3u64, 6u64, Adt11::C3, 7u64, 12u64)
                            } else {
                                Adt13::C0(3u64, 7u64, Adt11::C0, 3u64, 7u64)
                            }
                        } else {
                            let a1967 = v0;
                            let a1968 = 51u64;
                            if nat_lt(a1967, a1968) {
                                Adt13::C0(3u64, 8u64, Adt11::C1, 3u64, 7u64)
                            } else {
                                Adt13::C0(3u64, 9u64, Adt11::C2, 15u64, 3u64)
                            }
                        }
                    } else {
                        let a1969 = v0;
                        let a1970 = 54u64;
                        if nat_lt(a1969, a1970) {
                            let a1971 = v0;
                            let a1972 = 53u64;
                            if nat_lt(a1971, a1972) {
                                Adt13::C0(3u64, 10u64, Adt11::C3, 15u64, 8u64)
                            } else {
                                Adt13::C0(3u64, 11u64, Adt11::C0, 11u64, 15u64)
                            }
                        } else {
                            let a1973 = v0;
                            let a1974 = 55u64;
                            if nat_lt(a1973, a1974) {
                                Adt13::C0(3u64, 12u64, Adt11::C2, 7u64, 11u64)
                            } else {
                                Adt13::C0(3u64, 13u64, Adt11::C3, 7u64, 7u64)
                            }
                        }
                    }
                } else {
                    let a1975 = v0;
                    let a1976 = 60u64;
                    if nat_lt(a1975, a1976) {
                        let a1977 = v0;
                        let a1978 = 58u64;
                        if nat_lt(a1977, a1978) {
                            let a1979 = v0;
                            let a1980 = 57u64;
                            if nat_lt(a1979, a1980) {
                                Adt13::C0(4u64, 0u64, Adt11::C0, 0u64, 5u64)
                            } else {
                                Adt13::C0(4u64, 1u64, Adt11::C1, 0u64, 8u64)
                            }
                        } else {
                            let a1981 = v0;
                            let a1982 = 59u64;
                            if nat_lt(a1981, a1982) {
                                Adt13::C0(4u64, 2u64, Adt11::C2, 15u64, 0u64)
                            } else {
                                Adt13::C0(4u64, 3u64, Adt11::C3, 15u64, 16u64)
                            }
                        }
                    } else {
                        let a1983 = v0;
                        let a1984 = 62u64;
                        if nat_lt(a1983, a1984) {
                            let a1985 = v0;
                            let a1986 = 61u64;
                            if nat_lt(a1985, a1986) {
                                Adt13::C0(4u64, 4u64, Adt11::C0, 10u64, 15u64)
                            } else {
                                Adt13::C0(4u64, 5u64, Adt11::C2, 5u64, 10u64)
                            }
                        } else {
                            let a1987 = v0;
                            let a1988 = 63u64;
                            if nat_lt(a1987, a1988) {
                                Adt13::C0(4u64, 6u64, Adt11::C3, 5u64, 12u64)
                            } else {
                                Adt13::C0(4u64, 7u64, Adt11::C0, 0u64, 5u64)
                            }
                        }
                    }
                }
            }
        }
    } else {
        let a1989 = v0;
        let a1990 = 96u64;
        if nat_lt(a1989, a1990) {
            let a1991 = v0;
            let a1992 = 80u64;
            if nat_lt(a1991, a1992) {
                let a1993 = v0;
                let a1994 = 72u64;
                if nat_lt(a1993, a1994) {
                    let a1995 = v0;
                    let a1996 = 68u64;
                    if nat_lt(a1995, a1996) {
                        let a1997 = v0;
                        let a1998 = 66u64;
                        if nat_lt(a1997, a1998) {
                            let a1999 = v0;
                            let a2000 = 65u64;
                            if nat_lt(a1999, a2000) {
                                Adt13::C0(4u64, 8u64, Adt11::C1, 0u64, 9u64)
                            } else {
                                Adt13::C0(4u64, 9u64, Adt11::C2, 15u64, 0u64)
                            }
                        } else {
                            let a2001 = v0;
                            let a2002 = 67u64;
                            if nat_lt(a2001, a2002) {
                                Adt13::C0(4u64, 10u64, Adt11::C3, 15u64, 8u64)
                            } else {
                                Adt13::C0(4u64, 11u64, Adt11::C0, 10u64, 15u64)
                            }
                        }
                    } else {
                        let a2003 = v0;
                        let a2004 = 70u64;
                        if nat_lt(a2003, a2004) {
                            let a2005 = v0;
                            let a2006 = 69u64;
                            if nat_lt(a2005, a2006) {
                                Adt13::C0(4u64, 12u64, Adt11::C2, 5u64, 10u64)
                            } else {
                                Adt13::C0(4u64, 13u64, Adt11::C3, 5u64, 7u64)
                            }
                        } else {
                            let a2007 = v0;
                            let a2008 = 71u64;
                            if nat_lt(a2007, a2008) {
                                Adt13::C0(5u64, 0u64, Adt11::C0, 1u64, 6u64)
                            } else {
                                Adt13::C0(5u64, 1u64, Adt11::C1, 1u64, 10u64)
                            }
                        }
                    }
                } else {
                    let a2009 = v0;
                    let a2010 = 76u64;
                    if nat_lt(a2009, a2010) {
                        let a2011 = v0;
                        let a2012 = 74u64;
                        if nat_lt(a2011, a2012) {
                            let a2013 = v0;
                            let a2014 = 73u64;
                            if nat_lt(a2013, a2014) {
                                Adt13::C0(5u64, 2u64, Adt11::C2, 12u64, 1u64)
                            } else {
                                Adt13::C0(5u64, 3u64, Adt11::C3, 12u64, 16u64)
                            }
                        } else {
                            let a2015 = v0;
                            let a2016 = 75u64;
                            if nat_lt(a2015, a2016) {
                                Adt13::C0(5u64, 4u64, Adt11::C0, 11u64, 12u64)
                            } else {
                                Adt13::C0(5u64, 5u64, Adt11::C2, 6u64, 11u64)
                            }
                        }
                    } else {
                        let a2017 = v0;
                        let a2018 = 78u64;
                        if nat_lt(a2017, a2018) {
                            let a2019 = v0;
                            let a2020 = 77u64;
                            if nat_lt(a2019, a2020) {
                                Adt13::C0(5u64, 6u64, Adt11::C3, 6u64, 12u64)
                            } else {
                                Adt13::C0(5u64, 7u64, Adt11::C0, 1u64, 6u64)
                            }
                        } else {
                            let a2021 = v0;
                            let a2022 = 79u64;
                            if nat_lt(a2021, a2022) {
                                Adt13::C0(5u64, 8u64, Adt11::C1, 1u64, 11u64)
                            } else {
                                Adt13::C0(5u64, 9u64, Adt11::C2, 12u64, 1u64)
                            }
                        }
                    }
                }
            } else {
                let a2023 = v0;
                let a2024 = 88u64;
                if nat_lt(a2023, a2024) {
                    let a2025 = v0;
                    let a2026 = 84u64;
                    if nat_lt(a2025, a2026) {
                        let a2027 = v0;
                        let a2028 = 82u64;
                        if nat_lt(a2027, a2028) {
                            let a2029 = v0;
                            let a2030 = 81u64;
                            if nat_lt(a2029, a2030) {
                                Adt13::C0(5u64, 10u64, Adt11::C3, 12u64, 8u64)
                            } else {
                                Adt13::C0(5u64, 11u64, Adt11::C0, 11u64, 12u64)
                            }
                        } else {
                            let a2031 = v0;
                            let a2032 = 83u64;
                            if nat_lt(a2031, a2032) {
                                Adt13::C0(5u64, 12u64, Adt11::C2, 6u64, 11u64)
                            } else {
                                Adt13::C0(5u64, 13u64, Adt11::C3, 6u64, 7u64)
                            }
                        }
                    } else {
                        let a2033 = v0;
                        let a2034 = 86u64;
                        if nat_lt(a2033, a2034) {
                            let a2035 = v0;
                            let a2036 = 85u64;
                            if nat_lt(a2035, a2036) {
                                Adt13::C0(6u64, 0u64, Adt11::C0, 2u64, 7u64)
                            } else {
                                Adt13::C0(6u64, 1u64, Adt11::C1, 2u64, 12u64)
                            }
                        } else {
                            let a2037 = v0;
                            let a2038 = 87u64;
                            if nat_lt(a2037, a2038) {
                                Adt13::C0(6u64, 2u64, Adt11::C2, 13u64, 2u64)
                            } else {
                                Adt13::C0(6u64, 3u64, Adt11::C3, 13u64, 16u64)
                            }
                        }
                    }
                } else {
                    let a2039 = v0;
                    let a2040 = 92u64;
                    if nat_lt(a2039, a2040) {
                        let a2041 = v0;
                        let a2042 = 90u64;
                        if nat_lt(a2041, a2042) {
                            let a2043 = v0;
                            let a2044 = 89u64;
                            if nat_lt(a2043, a2044) {
                                Adt13::C0(6u64, 4u64, Adt11::C0, 8u64, 13u64)
                            } else {
                                Adt13::C0(6u64, 5u64, Adt11::C2, 7u64, 8u64)
                            }
                        } else {
                            let a2045 = v0;
                            let a2046 = 91u64;
                            if nat_lt(a2045, a2046) {
                                Adt13::C0(6u64, 6u64, Adt11::C3, 7u64, 12u64)
                            } else {
                                Adt13::C0(6u64, 7u64, Adt11::C0, 2u64, 7u64)
                            }
                        }
                    } else {
                        let a2047 = v0;
                        let a2048 = 94u64;
                        if nat_lt(a2047, a2048) {
                            let a2049 = v0;
                            let a2050 = 93u64;
                            if nat_lt(a2049, a2050) {
                                Adt13::C0(6u64, 8u64, Adt11::C1, 2u64, 13u64)
                            } else {
                                Adt13::C0(6u64, 9u64, Adt11::C2, 13u64, 2u64)
                            }
                        } else {
                            let a2051 = v0;
                            let a2052 = 95u64;
                            if nat_lt(a2051, a2052) {
                                Adt13::C0(6u64, 10u64, Adt11::C3, 13u64, 8u64)
                            } else {
                                Adt13::C0(6u64, 11u64, Adt11::C0, 8u64, 13u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a2053 = v0;
            let a2054 = 112u64;
            if nat_lt(a2053, a2054) {
                let a2055 = v0;
                let a2056 = 104u64;
                if nat_lt(a2055, a2056) {
                    let a2057 = v0;
                    let a2058 = 100u64;
                    if nat_lt(a2057, a2058) {
                        let a2059 = v0;
                        let a2060 = 98u64;
                        if nat_lt(a2059, a2060) {
                            let a2061 = v0;
                            let a2062 = 97u64;
                            if nat_lt(a2061, a2062) {
                                Adt13::C0(6u64, 12u64, Adt11::C2, 7u64, 8u64)
                            } else {
                                Adt13::C0(6u64, 13u64, Adt11::C3, 7u64, 7u64)
                            }
                        } else {
                            let a2063 = v0;
                            let a2064 = 99u64;
                            if nat_lt(a2063, a2064) {
                                Adt13::C0(7u64, 0u64, Adt11::C0, 3u64, 4u64)
                            } else {
                                Adt13::C0(7u64, 1u64, Adt11::C1, 3u64, 14u64)
                            }
                        }
                    } else {
                        let a2065 = v0;
                        let a2066 = 102u64;
                        if nat_lt(a2065, a2066) {
                            let a2067 = v0;
                            let a2068 = 101u64;
                            if nat_lt(a2067, a2068) {
                                Adt13::C0(7u64, 2u64, Adt11::C2, 14u64, 3u64)
                            } else {
                                Adt13::C0(7u64, 3u64, Adt11::C3, 14u64, 16u64)
                            }
                        } else {
                            let a2069 = v0;
                            let a2070 = 103u64;
                            if nat_lt(a2069, a2070) {
                                Adt13::C0(7u64, 4u64, Adt11::C0, 9u64, 14u64)
                            } else {
                                Adt13::C0(7u64, 5u64, Adt11::C2, 4u64, 9u64)
                            }
                        }
                    }
                } else {
                    let a2071 = v0;
                    let a2072 = 108u64;
                    if nat_lt(a2071, a2072) {
                        let a2073 = v0;
                        let a2074 = 106u64;
                        if nat_lt(a2073, a2074) {
                            let a2075 = v0;
                            let a2076 = 105u64;
                            if nat_lt(a2075, a2076) {
                                Adt13::C0(7u64, 6u64, Adt11::C3, 4u64, 12u64)
                            } else {
                                Adt13::C0(7u64, 7u64, Adt11::C0, 3u64, 4u64)
                            }
                        } else {
                            let a2077 = v0;
                            let a2078 = 107u64;
                            if nat_lt(a2077, a2078) {
                                Adt13::C0(7u64, 8u64, Adt11::C1, 3u64, 15u64)
                            } else {
                                Adt13::C0(7u64, 9u64, Adt11::C2, 14u64, 3u64)
                            }
                        }
                    } else {
                        let a2079 = v0;
                        let a2080 = 110u64;
                        if nat_lt(a2079, a2080) {
                            let a2081 = v0;
                            let a2082 = 109u64;
                            if nat_lt(a2081, a2082) {
                                Adt13::C0(7u64, 10u64, Adt11::C3, 14u64, 8u64)
                            } else {
                                Adt13::C0(7u64, 11u64, Adt11::C0, 9u64, 14u64)
                            }
                        } else {
                            let a2083 = v0;
                            let a2084 = 111u64;
                            if nat_lt(a2083, a2084) {
                                Adt13::C0(7u64, 12u64, Adt11::C2, 4u64, 9u64)
                            } else {
                                Adt13::C0(7u64, 13u64, Adt11::C3, 4u64, 7u64)
                            }
                        }
                    }
                }
            } else {
                let a2085 = v0;
                let a2086 = 120u64;
                if nat_lt(a2085, a2086) {
                    let a2087 = v0;
                    let a2088 = 116u64;
                    if nat_lt(a2087, a2088) {
                        let a2089 = v0;
                        let a2090 = 114u64;
                        if nat_lt(a2089, a2090) {
                            let a2091 = v0;
                            let a2092 = 113u64;
                            if nat_lt(a2091, a2092) {
                                Adt13::C0(0u64, 0u64, Adt11::C2, 0u64, 8u64)
                            } else {
                                Adt13::C0(0u64, 1u64, Adt11::C2, 1u64, 9u64)
                            }
                        } else {
                            let a2093 = v0;
                            let a2094 = 115u64;
                            if nat_lt(a2093, a2094) {
                                Adt13::C0(0u64, 2u64, Adt11::C2, 2u64, 10u64)
                            } else {
                                Adt13::C0(0u64, 3u64, Adt11::C2, 3u64, 11u64)
                            }
                        }
                    } else {
                        let a2095 = v0;
                        let a2096 = 118u64;
                        if nat_lt(a2095, a2096) {
                            let a2097 = v0;
                            let a2098 = 117u64;
                            if nat_lt(a2097, a2098) {
                                Adt13::C0(0u64, 4u64, Adt11::C2, 4u64, 12u64)
                            } else {
                                Adt13::C0(0u64, 5u64, Adt11::C2, 5u64, 13u64)
                            }
                        } else {
                            let a2099 = v0;
                            let a2100 = 119u64;
                            if nat_lt(a2099, a2100) {
                                Adt13::C0(0u64, 6u64, Adt11::C2, 6u64, 14u64)
                            } else {
                                Adt13::C0(0u64, 7u64, Adt11::C2, 7u64, 15u64)
                            }
                        }
                    }
                } else {
                    let a2101 = v0;
                    let a2102 = 124u64;
                    if nat_lt(a2101, a2102) {
                        let a2103 = v0;
                        let a2104 = 122u64;
                        if nat_lt(a2103, a2104) {
                            let a2105 = v0;
                            let a2106 = 121u64;
                            if nat_lt(a2105, a2106) {
                                Adt13::C0(0u64, 8u64, Adt11::C4, 8u64, 0u64)
                            } else {
                                Adt13::C0(0u64, 9u64, Adt11::C4, 9u64, 1u64)
                            }
                        } else {
                            let a2107 = v0;
                            let a2108 = 123u64;
                            if nat_lt(a2107, a2108) {
                                Adt13::C0(0u64, 10u64, Adt11::C4, 10u64, 2u64)
                            } else {
                                Adt13::C0(0u64, 11u64, Adt11::C4, 11u64, 3u64)
                            }
                        }
                    } else {
                        let a2109 = v0;
                        let a2110 = 126u64;
                        if nat_lt(a2109, a2110) {
                            let a2111 = v0;
                            let a2112 = 125u64;
                            if nat_lt(a2111, a2112) {
                                Adt13::C0(0u64, 12u64, Adt11::C4, 12u64, 4u64)
                            } else {
                                Adt13::C0(0u64, 13u64, Adt11::C4, 13u64, 5u64)
                            }
                        } else {
                            let a2113 = v0;
                            let a2114 = 127u64;
                            if nat_lt(a2113, a2114) {
                                Adt13::C0(0u64, 14u64, Adt11::C4, 14u64, 6u64)
                            } else {
                                Adt13::C0(0u64, 15u64, Adt11::C4, 15u64, 7u64)
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn f78(v0: Adt4, v1: u64) -> u32 {
    let a2115 = v1;
    let a2116 = 8u64;
    if nat_lt(a2115, a2116) {
        let a2117 = v1;
        let a2118 = 4u64;
        if nat_lt(a2117, a2118) {
            let a2119 = v1;
            let a2120 = 2u64;
            if nat_lt(a2119, a2120) {
                let a2121 = v1;
                let a2122 = 1u64;
                if nat_lt(a2121, a2122) {
                    match v0 {
                        Adt4::C0(h2123, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2123
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, h2124, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2124
                        }
                    }
                }
            } else {
                let a2125 = v1;
                let a2126 = 3u64;
                if nat_lt(a2125, a2126) {
                    match v0 {
                        Adt4::C0(_, _, h2127, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2127
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, h2128, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2128
                        }
                    }
                }
            }
        } else {
            let a2129 = v1;
            let a2130 = 6u64;
            if nat_lt(a2129, a2130) {
                let a2131 = v1;
                let a2132 = 5u64;
                if nat_lt(a2131, a2132) {
                    match v0 {
                        Adt4::C0(_, _, _, _, h2133, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2133
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, h2134, _, _, _, _, _, _, _, _, _, _) => {
                            h2134
                        }
                    }
                }
            } else {
                let a2135 = v1;
                let a2136 = 7u64;
                if nat_lt(a2135, a2136) {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, h2137, _, _, _, _, _, _, _, _, _) => {
                            h2137
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, h2138, _, _, _, _, _, _, _, _) => {
                            h2138
                        }
                    }
                }
            }
        }
    } else {
        let a2139 = v1;
        let a2140 = 12u64;
        if nat_lt(a2139, a2140) {
            let a2141 = v1;
            let a2142 = 10u64;
            if nat_lt(a2141, a2142) {
                let a2143 = v1;
                let a2144 = 9u64;
                if nat_lt(a2143, a2144) {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, h2145, _, _, _, _, _, _, _) => {
                            h2145
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h2146, _, _, _, _, _, _) => {
                            h2146
                        }
                    }
                }
            } else {
                let a2147 = v1;
                let a2148 = 11u64;
                if nat_lt(a2147, a2148) {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h2149, _, _, _, _, _) => {
                            h2149
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h2150, _, _, _, _) => {
                            h2150
                        }
                    }
                }
            }
        } else {
            let a2151 = v1;
            let a2152 = 14u64;
            if nat_lt(a2151, a2152) {
                let a2153 = v1;
                let a2154 = 13u64;
                if nat_lt(a2153, a2154) {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2155, _, _, _) => {
                            h2155
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2156, _, _) => {
                            h2156
                        }
                    }
                }
            } else {
                let a2157 = v1;
                let a2158 = 15u64;
                if nat_lt(a2157, a2158) {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2159, _) => {
                            h2159
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2160) => {
                            h2160
                        }
                    }
                }
            }
        }
    }
}

pub fn f79(v0: u64, v1: u64) -> R<u64> {
    let v2: u64 = {
        let a2163 = {
            let a2161 = v0;
            let a2162 = 16u64;
            nat_mul(a2161, a2162)?
        };
        let a2164 = v1;
        nat_add(a2163, a2164)?
    };
    let a2165 = v2;
    let a2166 = 56u64;
    if nat_lt(a2165, a2166) {
        let a2167 = v2;
        let a2168 = 28u64;
        if nat_lt(a2167, a2168) {
            let a2169 = v2;
            let a2170 = 14u64;
            if nat_lt(a2169, a2170) {
                let a2171 = v2;
                let a2172 = 7u64;
                if nat_lt(a2171, a2172) {
                    let a2173 = v2;
                    let a2174 = 3u64;
                    if nat_lt(a2173, a2174) {
                        let a2175 = v2;
                        let a2176 = 1u64;
                        if nat_lt(a2175, a2176) {
                            Ok(0u64)
                        } else {
                            let a2177 = v2;
                            let a2178 = 2u64;
                            if nat_lt(a2177, a2178) {
                                Ok(1u64)
                            } else {
                                Ok(2u64)
                            }
                        }
                    } else {
                        let a2179 = v2;
                        let a2180 = 5u64;
                        if nat_lt(a2179, a2180) {
                            let a2181 = v2;
                            let a2182 = 4u64;
                            if nat_lt(a2181, a2182) {
                                Ok(3u64)
                            } else {
                                Ok(4u64)
                            }
                        } else {
                            let a2183 = v2;
                            let a2184 = 6u64;
                            if nat_lt(a2183, a2184) {
                                Ok(5u64)
                            } else {
                                Ok(6u64)
                            }
                        }
                    }
                } else {
                    let a2185 = v2;
                    let a2186 = 10u64;
                    if nat_lt(a2185, a2186) {
                        let a2187 = v2;
                        let a2188 = 8u64;
                        if nat_lt(a2187, a2188) {
                            Ok(7u64)
                        } else {
                            let a2189 = v2;
                            let a2190 = 9u64;
                            if nat_lt(a2189, a2190) {
                                Ok(8u64)
                            } else {
                                Ok(9u64)
                            }
                        }
                    } else {
                        let a2191 = v2;
                        let a2192 = 12u64;
                        if nat_lt(a2191, a2192) {
                            let a2193 = v2;
                            let a2194 = 11u64;
                            if nat_lt(a2193, a2194) {
                                Ok(10u64)
                            } else {
                                Ok(11u64)
                            }
                        } else {
                            let a2195 = v2;
                            let a2196 = 13u64;
                            if nat_lt(a2195, a2196) {
                                Ok(12u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    }
                }
            } else {
                let a2197 = v2;
                let a2198 = 21u64;
                if nat_lt(a2197, a2198) {
                    let a2199 = v2;
                    let a2200 = 17u64;
                    if nat_lt(a2199, a2200) {
                        let a2201 = v2;
                        let a2202 = 15u64;
                        if nat_lt(a2201, a2202) {
                            Ok(14u64)
                        } else {
                            let a2203 = v2;
                            let a2204 = 16u64;
                            if nat_lt(a2203, a2204) {
                                Ok(15u64)
                            } else {
                                Ok(2u64)
                            }
                        }
                    } else {
                        let a2205 = v2;
                        let a2206 = 19u64;
                        if nat_lt(a2205, a2206) {
                            let a2207 = v2;
                            let a2208 = 18u64;
                            if nat_lt(a2207, a2208) {
                                Ok(6u64)
                            } else {
                                Ok(3u64)
                            }
                        } else {
                            let a2209 = v2;
                            let a2210 = 20u64;
                            if nat_lt(a2209, a2210) {
                                Ok(10u64)
                            } else {
                                Ok(7u64)
                            }
                        }
                    }
                } else {
                    let a2211 = v2;
                    let a2212 = 24u64;
                    if nat_lt(a2211, a2212) {
                        let a2213 = v2;
                        let a2214 = 22u64;
                        if nat_lt(a2213, a2214) {
                            Ok(0u64)
                        } else {
                            let a2215 = v2;
                            let a2216 = 23u64;
                            if nat_lt(a2215, a2216) {
                                Ok(4u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    } else {
                        let a2217 = v2;
                        let a2218 = 26u64;
                        if nat_lt(a2217, a2218) {
                            let a2219 = v2;
                            let a2220 = 25u64;
                            if nat_lt(a2219, a2220) {
                                Ok(1u64)
                            } else {
                                Ok(11u64)
                            }
                        } else {
                            let a2221 = v2;
                            let a2222 = 27u64;
                            if nat_lt(a2221, a2222) {
                                Ok(12u64)
                            } else {
                                Ok(5u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a2223 = v2;
            let a2224 = 42u64;
            if nat_lt(a2223, a2224) {
                let a2225 = v2;
                let a2226 = 35u64;
                if nat_lt(a2225, a2226) {
                    let a2227 = v2;
                    let a2228 = 31u64;
                    if nat_lt(a2227, a2228) {
                        let a2229 = v2;
                        let a2230 = 29u64;
                        if nat_lt(a2229, a2230) {
                            Ok(9u64)
                        } else {
                            let a2231 = v2;
                            let a2232 = 30u64;
                            if nat_lt(a2231, a2232) {
                                Ok(14u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    } else {
                        let a2233 = v2;
                        let a2234 = 33u64;
                        if nat_lt(a2233, a2234) {
                            let a2235 = v2;
                            let a2236 = 32u64;
                            if nat_lt(a2235, a2236) {
                                Ok(8u64)
                            } else {
                                Ok(3u64)
                            }
                        } else {
                            let a2237 = v2;
                            let a2238 = 34u64;
                            if nat_lt(a2237, a2238) {
                                Ok(4u64)
                            } else {
                                Ok(10u64)
                            }
                        }
                    }
                } else {
                    let a2239 = v2;
                    let a2240 = 38u64;
                    if nat_lt(a2239, a2240) {
                        let a2241 = v2;
                        let a2242 = 36u64;
                        if nat_lt(a2241, a2242) {
                            Ok(12u64)
                        } else {
                            let a2243 = v2;
                            let a2244 = 37u64;
                            if nat_lt(a2243, a2244) {
                                Ok(13u64)
                            } else {
                                Ok(2u64)
                            }
                        }
                    } else {
                        let a2245 = v2;
                        let a2246 = 40u64;
                        if nat_lt(a2245, a2246) {
                            let a2247 = v2;
                            let a2248 = 39u64;
                            if nat_lt(a2247, a2248) {
                                Ok(7u64)
                            } else {
                                Ok(14u64)
                            }
                        } else {
                            let a2249 = v2;
                            let a2250 = 41u64;
                            if nat_lt(a2249, a2250) {
                                Ok(6u64)
                            } else {
                                Ok(5u64)
                            }
                        }
                    }
                }
            } else {
                let a2251 = v2;
                let a2252 = 49u64;
                if nat_lt(a2251, a2252) {
                    let a2253 = v2;
                    let a2254 = 45u64;
                    if nat_lt(a2253, a2254) {
                        let a2255 = v2;
                        let a2256 = 43u64;
                        if nat_lt(a2255, a2256) {
                            Ok(9u64)
                        } else {
                            let a2257 = v2;
                            let a2258 = 44u64;
                            if nat_lt(a2257, a2258) {
                                Ok(0u64)
                            } else {
                                Ok(11u64)
                            }
                        }
                    } else {
                        let a2259 = v2;
                        let a2260 = 47u64;
                        if nat_lt(a2259, a2260) {
                            let a2261 = v2;
                            let a2262 = 46u64;
                            if nat_lt(a2261, a2262) {
                                Ok(15u64)
                            } else {
                                Ok(8u64)
                            }
                        } else {
                            let a2263 = v2;
                            let a2264 = 48u64;
                            if nat_lt(a2263, a2264) {
                                Ok(1u64)
                            } else {
                                Ok(10u64)
                            }
                        }
                    }
                } else {
                    let a2265 = v2;
                    let a2266 = 52u64;
                    if nat_lt(a2265, a2266) {
                        let a2267 = v2;
                        let a2268 = 50u64;
                        if nat_lt(a2267, a2268) {
                            Ok(7u64)
                        } else {
                            let a2269 = v2;
                            let a2270 = 51u64;
                            if nat_lt(a2269, a2270) {
                                Ok(12u64)
                            } else {
                                Ok(9u64)
                            }
                        }
                    } else {
                        let a2271 = v2;
                        let a2272 = 54u64;
                        if nat_lt(a2271, a2272) {
                            let a2273 = v2;
                            let a2274 = 53u64;
                            if nat_lt(a2273, a2274) {
                                Ok(14u64)
                            } else {
                                Ok(3u64)
                            }
                        } else {
                            let a2275 = v2;
                            let a2276 = 55u64;
                            if nat_lt(a2275, a2276) {
                                Ok(13u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    }
                }
            }
        }
    } else {
        let a2277 = v2;
        let a2278 = 84u64;
        if nat_lt(a2277, a2278) {
            let a2279 = v2;
            let a2280 = 70u64;
            if nat_lt(a2279, a2280) {
                let a2281 = v2;
                let a2282 = 63u64;
                if nat_lt(a2281, a2282) {
                    let a2283 = v2;
                    let a2284 = 59u64;
                    if nat_lt(a2283, a2284) {
                        let a2285 = v2;
                        let a2286 = 57u64;
                        if nat_lt(a2285, a2286) {
                            Ok(4u64)
                        } else {
                            let a2287 = v2;
                            let a2288 = 58u64;
                            if nat_lt(a2287, a2288) {
                                Ok(0u64)
                            } else {
                                Ok(11u64)
                            }
                        }
                    } else {
                        let a2289 = v2;
                        let a2290 = 61u64;
                        if nat_lt(a2289, a2290) {
                            let a2291 = v2;
                            let a2292 = 60u64;
                            if nat_lt(a2291, a2292) {
                                Ok(2u64)
                            } else {
                                Ok(5u64)
                            }
                        } else {
                            let a2293 = v2;
                            let a2294 = 62u64;
                            if nat_lt(a2293, a2294) {
                                Ok(8u64)
                            } else {
                                Ok(1u64)
                            }
                        }
                    }
                } else {
                    let a2295 = v2;
                    let a2296 = 66u64;
                    if nat_lt(a2295, a2296) {
                        let a2297 = v2;
                        let a2298 = 64u64;
                        if nat_lt(a2297, a2298) {
                            Ok(6u64)
                        } else {
                            let a2299 = v2;
                            let a2300 = 65u64;
                            if nat_lt(a2299, a2300) {
                                Ok(12u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    } else {
                        let a2301 = v2;
                        let a2302 = 68u64;
                        if nat_lt(a2301, a2302) {
                            let a2303 = v2;
                            let a2304 = 67u64;
                            if nat_lt(a2303, a2304) {
                                Ok(9u64)
                            } else {
                                Ok(11u64)
                            }
                        } else {
                            let a2305 = v2;
                            let a2306 = 69u64;
                            if nat_lt(a2305, a2306) {
                                Ok(15u64)
                            } else {
                                Ok(10u64)
                            }
                        }
                    }
                }
            } else {
                let a2307 = v2;
                let a2308 = 77u64;
                if nat_lt(a2307, a2308) {
                    let a2309 = v2;
                    let a2310 = 73u64;
                    if nat_lt(a2309, a2310) {
                        let a2311 = v2;
                        let a2312 = 71u64;
                        if nat_lt(a2311, a2312) {
                            Ok(14u64)
                        } else {
                            let a2313 = v2;
                            let a2314 = 72u64;
                            if nat_lt(a2313, a2314) {
                                Ok(8u64)
                            } else {
                                Ok(7u64)
                            }
                        }
                    } else {
                        let a2315 = v2;
                        let a2316 = 75u64;
                        if nat_lt(a2315, a2316) {
                            let a2317 = v2;
                            let a2318 = 74u64;
                            if nat_lt(a2317, a2318) {
                                Ok(2u64)
                            } else {
                                Ok(5u64)
                            }
                        } else {
                            let a2319 = v2;
                            let a2320 = 76u64;
                            if nat_lt(a2319, a2320) {
                                Ok(3u64)
                            } else {
                                Ok(0u64)
                            }
                        }
                    }
                } else {
                    let a2321 = v2;
                    let a2322 = 80u64;
                    if nat_lt(a2321, a2322) {
                        let a2323 = v2;
                        let a2324 = 78u64;
                        if nat_lt(a2323, a2324) {
                            Ok(1u64)
                        } else {
                            let a2325 = v2;
                            let a2326 = 79u64;
                            if nat_lt(a2325, a2326) {
                                Ok(6u64)
                            } else {
                                Ok(4u64)
                            }
                        }
                    } else {
                        let a2327 = v2;
                        let a2328 = 82u64;
                        if nat_lt(a2327, a2328) {
                            let a2329 = v2;
                            let a2330 = 81u64;
                            if nat_lt(a2329, a2330) {
                                Ok(9u64)
                            } else {
                                Ok(14u64)
                            }
                        } else {
                            let a2331 = v2;
                            let a2332 = 83u64;
                            if nat_lt(a2331, a2332) {
                                Ok(11u64)
                            } else {
                                Ok(5u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a2333 = v2;
            let a2334 = 98u64;
            if nat_lt(a2333, a2334) {
                let a2335 = v2;
                let a2336 = 91u64;
                if nat_lt(a2335, a2336) {
                    let a2337 = v2;
                    let a2338 = 87u64;
                    if nat_lt(a2337, a2338) {
                        let a2339 = v2;
                        let a2340 = 85u64;
                        if nat_lt(a2339, a2340) {
                            Ok(8u64)
                        } else {
                            let a2341 = v2;
                            let a2342 = 86u64;
                            if nat_lt(a2341, a2342) {
                                Ok(12u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    } else {
                        let a2343 = v2;
                        let a2344 = 89u64;
                        if nat_lt(a2343, a2344) {
                            let a2345 = v2;
                            let a2346 = 88u64;
                            if nat_lt(a2345, a2346) {
                                Ok(1u64)
                            } else {
                                Ok(13u64)
                            }
                        } else {
                            let a2347 = v2;
                            let a2348 = 90u64;
                            if nat_lt(a2347, a2348) {
                                Ok(3u64)
                            } else {
                                Ok(0u64)
                            }
                        }
                    }
                } else {
                    let a2349 = v2;
                    let a2350 = 94u64;
                    if nat_lt(a2349, a2350) {
                        let a2351 = v2;
                        let a2352 = 92u64;
                        if nat_lt(a2351, a2352) {
                            Ok(10u64)
                        } else {
                            let a2353 = v2;
                            let a2354 = 93u64;
                            if nat_lt(a2353, a2354) {
                                Ok(2u64)
                            } else {
                                Ok(6u64)
                            }
                        }
                    } else {
                        let a2355 = v2;
                        let a2356 = 96u64;
                        if nat_lt(a2355, a2356) {
                            let a2357 = v2;
                            let a2358 = 95u64;
                            if nat_lt(a2357, a2358) {
                                Ok(4u64)
                            } else {
                                Ok(7u64)
                            }
                        } else {
                            let a2359 = v2;
                            let a2360 = 97u64;
                            if nat_lt(a2359, a2360) {
                                Ok(11u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    }
                }
            } else {
                let a2361 = v2;
                let a2362 = 105u64;
                if nat_lt(a2361, a2362) {
                    let a2363 = v2;
                    let a2364 = 101u64;
                    if nat_lt(a2363, a2364) {
                        let a2365 = v2;
                        let a2366 = 99u64;
                        if nat_lt(a2365, a2366) {
                            Ok(5u64)
                        } else {
                            let a2367 = v2;
                            let a2368 = 100u64;
                            if nat_lt(a2367, a2368) {
                                Ok(0u64)
                            } else {
                                Ok(1u64)
                            }
                        }
                    } else {
                        let a2369 = v2;
                        let a2370 = 103u64;
                        if nat_lt(a2369, a2370) {
                            let a2371 = v2;
                            let a2372 = 102u64;
                            if nat_lt(a2371, a2372) {
                                Ok(9u64)
                            } else {
                                Ok(8u64)
                            }
                        } else {
                            let a2373 = v2;
                            let a2374 = 104u64;
                            if nat_lt(a2373, a2374) {
                                Ok(6u64)
                            } else {
                                Ok(14u64)
                            }
                        }
                    }
                } else {
                    let a2375 = v2;
                    let a2376 = 108u64;
                    if nat_lt(a2375, a2376) {
                        let a2377 = v2;
                        let a2378 = 106u64;
                        if nat_lt(a2377, a2378) {
                            Ok(10u64)
                        } else {
                            let a2379 = v2;
                            let a2380 = 107u64;
                            if nat_lt(a2379, a2380) {
                                Ok(2u64)
                            } else {
                                Ok(12u64)
                            }
                        }
                    } else {
                        let a2381 = v2;
                        let a2382 = 110u64;
                        if nat_lt(a2381, a2382) {
                            let a2383 = v2;
                            let a2384 = 109u64;
                            if nat_lt(a2383, a2384) {
                                Ok(3u64)
                            } else {
                                Ok(4u64)
                            }
                        } else {
                            let a2385 = v2;
                            let a2386 = 111u64;
                            if nat_lt(a2385, a2386) {
                                Ok(7u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn f80(v0: u64) -> u32 {
    let a2387 = v0;
    let a2388 = 12u64;
    if nat_lt(a2387, a2388) {
        let a2389 = v0;
        let a2390 = 9u64;
        if nat_lt(a2389, a2390) {
            let a2391 = v0;
            let a2392 = 8u64;
            if nat_lt(a2391, a2392) {
                7u32
            } else {
                8u32
            }
        } else {
            let a2393 = v0;
            let a2394 = 10u64;
            if nat_lt(a2393, a2394) {
                9u32
            } else {
                let a2395 = v0;
                let a2396 = 11u64;
                if nat_lt(a2395, a2396) {
                    10u32
                } else {
                    11u32
                }
            }
        }
    } else {
        let a2397 = v0;
        let a2398 = 14u64;
        if nat_lt(a2397, a2398) {
            let a2399 = v0;
            let a2400 = 13u64;
            if nat_lt(a2399, a2400) {
                12u32
            } else {
                13u32
            }
        } else {
            let a2401 = v0;
            let a2402 = 15u64;
            if nat_lt(a2401, a2402) {
                14u32
            } else {
                let a2403 = v0;
                let a2404 = 16u64;
                if nat_lt(a2403, a2404) {
                    15u32
                } else {
                    16u32
                }
            }
        }
    }
}

pub fn f81(v0: Adt1, v1: u64) -> u32 {
    let a2405 = v1;
    let a2406 = 4u64;
    if nat_lt(a2405, a2406) {
        let a2407 = v1;
        let a2408 = 2u64;
        if nat_lt(a2407, a2408) {
            let a2409 = v1;
            let a2410 = 1u64;
            if nat_lt(a2409, a2410) {
                match v0 {
                    Adt1::C0(h2411, _, _, _, _, _, _, _) => {
                        h2411
                    }
                }
            } else {
                match v0 {
                    Adt1::C0(_, h2412, _, _, _, _, _, _) => {
                        h2412
                    }
                }
            }
        } else {
            let a2413 = v1;
            let a2414 = 3u64;
            if nat_lt(a2413, a2414) {
                match v0 {
                    Adt1::C0(_, _, h2415, _, _, _, _, _) => {
                        h2415
                    }
                }
            } else {
                match v0 {
                    Adt1::C0(_, _, _, h2416, _, _, _, _) => {
                        h2416
                    }
                }
            }
        }
    } else {
        let a2417 = v1;
        let a2418 = 6u64;
        if nat_lt(a2417, a2418) {
            let a2419 = v1;
            let a2420 = 5u64;
            if nat_lt(a2419, a2420) {
                match v0 {
                    Adt1::C0(_, _, _, _, h2421, _, _, _) => {
                        h2421
                    }
                }
            } else {
                match v0 {
                    Adt1::C0(_, _, _, _, _, h2422, _, _) => {
                        h2422
                    }
                }
            }
        } else {
            let a2423 = v1;
            let a2424 = 7u64;
            if nat_lt(a2423, a2424) {
                match v0 {
                    Adt1::C0(_, _, _, _, _, _, h2425, _) => {
                        h2425
                    }
                }
            } else {
                match v0 {
                    Adt1::C0(_, _, _, _, _, _, _, h2426) => {
                        h2426
                    }
                }
            }
        }
    }
}

pub fn f82(v0: Adt11, v1: u32, v2: u32) -> u32 {
    let m2427 = v0;
    match m2427 {
        Adt11::C0 => {
            f83(v1, v2)
        }
        Adt11::C1 => {
            f83(v1, v2)
        }
        Adt11::C2 => {
            f84(v1, v2)
        }
        Adt11::C3 => {
            f85(v1, v2)
        }
        Adt11::C4 => {
            f84(v1, v2)
        }
    }
}

pub fn f83(v0: u32, v1: u32) -> u32 {
    let m2430 = {
        let a2428 = f86(v0);
        let a2429 = f86(v1);
        fixed_u64::checked_add(a2428, a2429)
    };
    match m2430 {
        None => {
            0u32
        }
        Some(v2) => {
            f87(v2)
        }
    }
}

pub fn f84(v0: u32, v1: u32) -> u32 {
    let a2431 = v0;
    let a2432 = v1;
    fixed_u32::bit_xor(a2431, a2432)
}

pub fn f85(v0: u32, v1: u32) -> u32 {
    let m2435 = {
        let a2433 = 32u32;
        let a2434 = v1;
        fixed_u32::checked_sub(a2433, a2434)
    };
    match m2435 {
        None => {
            v0
        }
        Some(v2) => {
            f88(v0, v1, v2)
        }
    }
}

pub fn f86(v0: u32) -> u64 {
    let m2437 = {
        let a2436 = v0;
        fixed_u64::convert(i128::from(a2436))
    };
    match m2437 {
        None => {
            0u64
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f87(v0: u64) -> u32 {
    let m2441 = {
        let a2440 = {
            let a2438 = v0;
            let a2439 = 4294967295u64;
            fixed_u64::bit_and(a2438, a2439)
        };
        fixed_u32::convert(i128::from(a2440))
    };
    match m2441 {
        None => {
            0u32
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f88(v0: u32, v1: u32, v2: u32) -> u32 {
    let a2442 = f57(v0, v1);
    let a2443 = f69(v0, v2);
    fixed_u32::bit_or(a2442, a2443)
}

pub fn run(p0: Bytes, p1: Bytes, p2: u64) -> R<Bytes> {
    f0(p0, p1, p2)
}
