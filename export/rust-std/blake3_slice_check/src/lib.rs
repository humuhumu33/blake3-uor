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
    C0,
    C1(Bytes),
    C2(Bytes),
}

#[derive(Clone)]
pub enum Adt1 {
    C0(u64),
    C1,
    C2,
    C3,
    C4(u64),
    C5,
    C6,
}

#[derive(Clone)]
pub enum Adt2 {
    C0(Adt3, u32, u64, Bytes, List<Adt4>, List<Adt5>, List<Adt5>),
}

#[derive(Clone)]
pub enum Adt3 {
    C0(u32, u32, u32, u32, u32, u32, u32, u32),
}

#[derive(Clone)]
pub enum Adt4 {
    C0(Adt3, u64, u64),
}

#[derive(Clone)]
pub enum Adt5 {
    C0(Adt6),
    C1(Adt4, u64),
    C2(Adt4, u64),
}

#[derive(Clone)]
pub enum Adt6 {
    C0(u64, u64, u64, u64, u64, Adt3, Adt7, u32, u32, u32, u32, Adt7),
}

#[derive(Clone)]
pub enum Adt7 {
    C0(u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32),
}

#[derive(Clone)]
pub enum Adt8 {
    C0(u64),
}

#[derive(Clone)]
pub enum Adt9 {
    C0(u64, u64, u64, Adt3, bool, u64, bool),
}

#[derive(Clone)]
pub enum Adt10 {
    C0(u64, u64, u64, u64, u64, Adt3, Adt7, u64, u32, u32),
}

#[derive(Clone)]
pub enum Adt11 {
    C0(u64, u64, u64, Adt3, u32, List<Adt5>, List<Adt5>),
}

#[derive(Clone)]
pub enum Adt12 {
    C0(Adt3, Adt7, u32, u32, u32, u32, List<Adt13>),
}

#[derive(Clone)]
pub enum Adt13 {
    C0(u64, u64, u64, Adt14, u64, u64, u32, u32, u32),
}

#[derive(Clone)]
pub enum Adt14 {
    C0,
    C1,
    C2,
    C3,
    C4,
}

#[derive(Clone)]
pub enum Adt15 {
    C0(Adt7, Adt7, List<Adt13>),
}

#[derive(Clone)]
pub enum Adt16 {
    C0(Adt4, u64, List<Adt4>, List<Adt5>),
}

#[derive(Clone)]
pub enum Adt17 {
    C0(u64, u64, Adt14, u64, u64),
}

#[derive(Clone)]
pub enum Fn0 {
    F13(u32, u64, Adt3, Bytes),
}

impl Fn0 {
    pub fn apply(&self, p0: Adt9) -> R<Option<Adt9>> {
        match self {
            Fn0::F13(k0, k1, k2, k3) => f13(*k0, *k1, k2.clone(), k3.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn1 {
    F30(u32, u64, u64, Adt3, u64, Bytes),
}

impl Fn1 {
    pub fn apply(&self, p0: (u64, (Adt3, List<Adt5>))) -> R<(u64, (Adt3, List<Adt5>))> {
        match self {
            Fn1::F30(k0, k1, k2, k3, k4, k5) => f30(*k0, *k1, *k2, k3.clone(), *k4, k5.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn2 {
    F40(u64),
}

impl Fn2 {
    pub fn apply(&self, p0: Adt7, p1: Adt5) -> Adt7 {
        match self {
            Fn2::F40(k0) => f40(*k0, p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn3 {
    F43,
}

impl Fn3 {
    pub fn apply(&self, p0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
        match self {
            Fn3::F43 => f43(p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn4 {
    F51(u32, Adt3),
}

impl Fn4 {
    pub fn apply(&self, p0: (Adt10, (List<Adt5>, u64)), p1: Adt4) -> R<(Adt10, (List<Adt5>, u64))> {
        match self {
            Fn4::F51(k0, k1) => f51(*k0, k1.clone(), p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn5 {
    F53((Adt10, (List<Adt5>, u64))),
}

impl Fn5 {
    pub fn apply(&self, p0: (u64, List<Adt5>)) -> R<(u64, List<Adt5>)> {
        match self {
            Fn5::F53(k0) => f53(k0.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn6 {
    F57(u32, Bytes, Adt3, Adt2),
}

impl Fn6 {
    pub fn apply(&self, p0: (u64, (List<Adt4>, List<Adt5>))) -> R<(u64, (List<Adt4>, List<Adt5>))> {
        match self {
            Fn6::F57(k0, k1, k2, k3) => f57(*k0, k1.clone(), k2.clone(), k3.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn7 {
    F59(u64),
}

impl Fn7 {
    pub fn apply(&self, p0: u64) -> R<Option<u64>> {
        match self {
            Fn7::F59(k0) => f59(*k0, p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn8 {
    F61,
}

impl Fn8 {
    pub fn apply(&self, p0: Adt7, p1: Adt13) -> Adt7 {
        match self {
            Fn8::F61 => f61(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn9 {
    F63(Adt3, Adt7),
}

impl Fn9 {
    pub fn apply(&self, p0: (u64, Adt15)) -> R<(u64, Adt15)> {
        match self {
            Fn9::F63(k0, k1) => f63(k0.clone(), k1.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn10 {
    F67,
}

impl Fn10 {
    pub fn apply(&self, p0: List<Adt5>, p1: Adt5) -> List<Adt5> {
        match self {
            Fn10::F67 => f67(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn11 {
    F72(u32, Adt3),
}

impl Fn11 {
    pub fn apply(&self, p0: Adt16) -> R<Option<Adt16>> {
        match self {
            Fn11::F72(k0, k1) => f72(*k0, k1.clone(), p0),
        }
    }
}

pub fn f0(v0: Adt0, v1: Bytes, v2: u64, v3: u64, v4: Bytes) -> R<Result<Bytes, Adt1>> {
    let m1 = f1(v0)?;
    match m1 {
        Ok(v5) => {
            f2(match v5.clone() {
                Adt2::C0(h2, _, _, _, _, _, _) => {
                    h2
                }
            }, match v5 {
                Adt2::C0(_, h3, _, _, _, _, _) => {
                    h3
                }
            }, v1, v2, v3, v4)
        }
        Err(v6) => {
            let m4 = v6;
            match m4 {
                Adt8::C0(v7) => {
                    Ok(Err::<Bytes, Adt1>(Adt1::C0(v7)))
                }
            }
        }
    }
}

pub fn f1(v0: Adt0) -> R<Result<Adt2, Adt8>> {
    let m5 = v0;
    match m5 {
        Adt0::C0 => {
            Ok(Ok::<Adt2, Adt8>(f3(f4(), 0u32)))
        }
        Adt0::C1(v1) => {
            let a7 = {
                let a6 = v1.clone();
                length_bytes(a6)
            };
            let a8 = 32u64;
            if nat_eq(a7, a8) {
                Ok(Ok::<Adt2, Adt8>(f3(f5(v1)?, f6())))
            } else {
                Ok(Err::<Adt2, Adt8>(Adt8::C0({
                    let a9 = v1;
                    length_bytes(a9)
                })))
            }
        }
        Adt0::C2(v2) => {
            Ok(Ok::<Adt2, Adt8>(f7(v2)?))
        }
    }
}

pub fn f2(v0: Adt3, v1: u32, v2: Bytes, v3: u64, v4: u64, v5: Bytes) -> R<Result<Bytes, Adt1>> {
    let a10 = v3;
    let a11 = f8(v4)?;
    if nat_lt(a10, a11) {
        let a13 = {
            let a12 = v5.clone();
            length_bytes(a12)
        };
        let a14 = 32u64;
        if nat_eq(a13, a14) {
            let a16 = 8u64;
            let a17 = {
                let a15 = v2.clone();
                length_bytes(a15)
            };
            if if nat_le(a16, a17) {
                let a18 = f9(v2.clone(), 8u64, 0u64)?;
                let a19 = f10(v4);
                if equal(a18, a19) {
                    let a20 = f9(v2.clone(), 8u64, 4u64)?;
                    let a21 = f11(v4);
                    equal(a20, a21)
                } else {
                    false
                }
            } else {
                false
            } {
                let v6: Adt9 = {
                    let (h22, _) = f12(Fn0::F13(v1, v3, v0.clone(), v2.clone()), 64u64, Adt9::C0(0u64, f8(v4)?, 8u64, f5(v5)?, true, 0u64, false))?;
                    h22
                };
                if match v6.clone() {
                    Adt9::C0(_, _, _, _, _, _, h23) => {
                        h23
                    }
                } {
                    Ok(Err::<Bytes, Adt1>(Adt1::C4(match v6 {
                        Adt9::C0(_, _, _, _, _, h24, _) => {
                            h24
                        }
                    })))
                } else {
                    let v7: u64 = f14(v4, v3)?;
                    let v9: Bytes = {
                        let m33 = {
                            let a30 = v2.clone();
                            let a31 = match v6.clone() {
                                Adt9::C0(_, _, h25, _, _, _, _) => {
                                    h25
                                }
                            };
                            let a32 = {
                                let a28 = {
                                    let a26 = v2;
                                    length_bytes(a26)
                                };
                                let a29 = match v6.clone() {
                                    Adt9::C0(_, _, h27, _, _, _, _) => {
                                        h27
                                    }
                                };
                                nat_sub(a28, a29)
                            };
                            slice_bytes(a30, a31, a32)
                        };
                        match m33 {
                            None => {
                                Bytes::lit(&[])
                            }
                            Some(v8) => {
                                v8
                            }
                        }
                    };
                    let a35 = {
                        let a34 = v9.clone();
                        length_bytes(a34)
                    };
                    let a36 = v7;
                    if nat_eq(a35, a36) {
                        let v10: Adt10 = {
                            let (h37, _) = f15(v0, v1, v9.clone(), 0u64, v7, v3, List::<Adt5>::nil())?;
                            h37
                        };
                        let v11: Adt6 = f16(if match v6.clone() {
                            Adt9::C0(_, _, _, _, h38, _, _) => {
                                h38
                            }
                        } {
                            f17(v10, 0u64)
                        } else {
                            v10
                        })?;
                        if f18(f19(match v11 {
                            Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h39) => {
                                h39
                            }
                        }), match v6 {
                            Adt9::C0(_, _, _, h40, _, _, _) => {
                                h40
                            }
                        }) {
                            Ok(Ok::<Bytes, Adt1>(v9))
                        } else {
                            Ok(Err::<Bytes, Adt1>(Adt1::C6))
                        }
                    } else {
                        Ok(Err::<Bytes, Adt1>(Adt1::C5))
                    }
                }
            } else {
                Ok(Err::<Bytes, Adt1>(Adt1::C2))
            }
        } else {
            Ok(Err::<Bytes, Adt1>(Adt1::C3))
        }
    } else {
        Ok(Err::<Bytes, Adt1>(Adt1::C1))
    }
}

pub fn f3(v0: Adt3, v1: u32) -> Adt2 {
    Adt2::C0(v0, v1, 0u64, Bytes::lit(&[]), List::<Adt4>::nil(), List::<Adt5>::nil(), List::<Adt5>::nil())
}

pub fn f4() -> Adt3 {
    Adt3::C0(1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, 1359893119u32, 2600822924u32, 528734635u32, 1541459225u32)
}

pub fn f5(v0: Bytes) -> R<Adt3> {
    Ok(f19(f20(v0.clone(), {
        let a41 = v0;
        length_bytes(a41)
    }, 0u64)?))
}

pub fn f6() -> u32 {
    16u32
}

pub fn f7(v0: Bytes) -> R<Adt2> {
    let v1: Adt11 = f21(v0)?;
    Ok(Adt2::C0(f19(f22(v1.clone(), 0u64)), f23(), 0u64, Bytes::lit(&[]), List::<Adt4>::nil(), List::<Adt5>::nil(), match v1 {
        Adt11::C0(_, _, _, _, _, _, h42) => {
            h42
        }
    }))
}

pub fn f8(v0: u64) -> R<u64> {
    let a43 = v0;
    let a44 = 0u64;
    if nat_eq(a43, a44) {
        Ok(1u64)
    } else {
        let a50 = {
            let a47 = {
                let a45 = v0;
                let a46 = 1u64;
                nat_sub(a45, a46)
            };
            let a48 = 1024u64;
            let a49 = 0u64;
            nat_quot(a47, a48, a49)
        };
        let a51 = 1u64;
        nat_add(a50, a51)
    }
}

pub fn f9(v0: Bytes, v1: u64, v2: u64) -> R<u32> {
    Ok(f24(f25(v0.clone(), v1, v2), f25(v0.clone(), v1, {
        let a52 = v2;
        let a53 = 1u64;
        nat_add(a52, a53)?
    }), f25(v0.clone(), v1, {
        let a54 = v2;
        let a55 = 2u64;
        nat_add(a54, a55)?
    }), f25(v0, v1, {
        let a56 = v2;
        let a57 = 3u64;
        nat_add(a56, a57)?
    })))
}

pub fn f10(v0: u64) -> u32 {
    f26({
        let a58 = v0;
        let a59 = 4294967296u64;
        let a60 = 0u64;
        nat_rem(a58, a59, a60)
    }, 32u64)
}

pub fn f11(v0: u64) -> u32 {
    let a61 = v0;
    let a62 = 4294967296u64;
    if nat_lt(a61, a62) {
        0u32
    } else {
        f26({
            let a63 = v0;
            let a64 = 4294967296u64;
            let a65 = 0u64;
            nat_quot(a63, a64, a65)
        }, 32u64)
    }
}

pub fn f12(v0: Fn0, p70: u64, p71: Adt9) -> R<(Adt9, bool)> {
    let m69 = (p70, p71);
    {
        let mut m69 = m69;
        loop {
            let step = {
                let (v1, v2) = m69;
                let m66 = v1;
                if m66 == 0 {
                    Err::<(u64, Adt9), R<(Adt9, bool)>>(Ok((v2, false)))
                } else {
                    let v3 = m66 - 1;
                    let m68 = {
                        let c67: Fn0 = v0.clone();
                        c67.apply(v2.clone())?
                    };
                    match m68 {
                        None => {
                            Err::<(u64, Adt9), R<(Adt9, bool)>>(Ok((v2, true)))
                        }
                        Some(v4) => {
                            Ok::<(u64, Adt9), R<(Adt9, bool)>>((v3, v4))
                        }
                    }
                }
            };
            match step {
                Ok(next) => { m69 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f13(v0: u32, v1: u64, v2: Adt3, v3: Bytes, v4: Adt9) -> R<Option<Adt9>> {
    f27(v2, v0, v3, v1, v4)
}

pub fn f14(v0: u64, v1: u64) -> R<u64> {
    let v2: u64 = {
        let a72 = v1;
        let a73 = 1024u64;
        nat_mul(a72, a73)?
    };
    let a76 = {
        let a74 = v2;
        let a75 = 1024u64;
        nat_add(a74, a75)?
    };
    let a77 = v0;
    if nat_le(a76, a77) {
        Ok(1024u64)
    } else {
        let a78 = v0;
        let a79 = v2;
        Ok(nat_sub(a78, a79))
    }
}

pub fn f15(v0: Adt3, v1: u32, v2: Bytes, v3: u64, v4: u64, v5: u64, v6: List<Adt5>) -> R<(Adt10, List<Adt5>)> {
    let v7: u64 = {
        let a80 = v3;
        let a81 = v4;
        nat_add(a80, a81)?
    };
    let v8: u64 = f28(v4)?;
    let v9: (u64, (Adt3, List<Adt5>)) = f29(Fn1::F30(v1, v3, v5, v0.clone(), v7, v2.clone()), {
        let a82 = v8;
        let a83 = 64u64;
        let a84 = 0u64;
        nat_quot(a82, a83, a84)
    }, (0u64, (v0.clone(), v6)))?;
    Ok((f31(v0, v1, v2, v7, v3, v5, v8, {
        let (h85, _) = {
            let (_, h86) = v9.clone();
            h86
        };
        h85
    }, {
        let a87 = v4;
        let a88 = v8;
        nat_sub(a87, a88)
    }, {
        let a89 = f32();
        let a90 = f33(v8);
        fixed_u32::bit_or(a89, a90)
    })?, {
        let (_, h91) = {
            let (_, h92) = v9;
            h92
        };
        h91
    }))
}

pub fn f16(v0: Adt10) -> R<Adt6> {
    let v1: u32 = f10(match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, _, h93, _, _) => {
            h93
        }
    });
    let v2: u32 = f11(match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, _, h94, _, _) => {
            h94
        }
    });
    Ok(Adt6::C0(match v0.clone() {
        Adt10::C0(h95, _, _, _, _, _, _, _, _, _) => {
            h95
        }
    }, match v0.clone() {
        Adt10::C0(_, h96, _, _, _, _, _, _, _, _) => {
            h96
        }
    }, match v0.clone() {
        Adt10::C0(_, _, h97, _, _, _, _, _, _, _) => {
            h97
        }
    }, match v0.clone() {
        Adt10::C0(_, _, _, h98, _, _, _, _, _, _) => {
            h98
        }
    }, match v0.clone() {
        Adt10::C0(_, _, _, _, h99, _, _, _, _, _) => {
            h99
        }
    }, match v0.clone() {
        Adt10::C0(_, _, _, _, _, h100, _, _, _, _) => {
            h100
        }
    }, match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, h101, _, _, _) => {
            h101
        }
    }, v1, v2, match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, _, _, h102, _) => {
            h102
        }
    }, match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, _, _, _, h103) => {
            h103
        }
    }, f34(match v0.clone() {
        Adt10::C0(_, _, _, _, _, h104, _, _, _, _) => {
            h104
        }
    }, match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, h105, _, _, _) => {
            h105
        }
    }, v1, v2, match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, _, _, h106, _) => {
            h106
        }
    }, match v0 {
        Adt10::C0(_, _, _, _, _, _, _, _, _, h107) => {
            h107
        }
    })?))
}

pub fn f17(v0: Adt10, v1: u64) -> Adt10 {
    Adt10::C0(2u64, match v0.clone() {
        Adt10::C0(_, h108, _, _, _, _, _, _, _, _) => {
            h108
        }
    }, match v0.clone() {
        Adt10::C0(_, _, h109, _, _, _, _, _, _, _) => {
            h109
        }
    }, v1, match v0.clone() {
        Adt10::C0(_, _, _, _, h110, _, _, _, _, _) => {
            h110
        }
    }, match v0.clone() {
        Adt10::C0(_, _, _, _, _, h111, _, _, _, _) => {
            h111
        }
    }, match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, h112, _, _, _) => {
            h112
        }
    }, v1, match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, _, _, h113, _) => {
            h113
        }
    }, {
        let a115 = match v0 {
            Adt10::C0(_, _, _, _, _, _, _, _, _, h114) => {
                h114
            }
        };
        let a116 = f35();
        fixed_u32::bit_or(a115, a116)
    })
}

pub fn f18(v0: Adt3, v1: Adt3) -> bool {
    let a119 = match v0.clone() {
        Adt3::C0(h117, _, _, _, _, _, _, _) => {
            h117
        }
    };
    let a120 = match v1.clone() {
        Adt3::C0(h118, _, _, _, _, _, _, _) => {
            h118
        }
    };
    if equal(a119, a120) {
        let a123 = match v0.clone() {
            Adt3::C0(_, h121, _, _, _, _, _, _) => {
                h121
            }
        };
        let a124 = match v1.clone() {
            Adt3::C0(_, h122, _, _, _, _, _, _) => {
                h122
            }
        };
        if equal(a123, a124) {
            let a127 = match v0.clone() {
                Adt3::C0(_, _, h125, _, _, _, _, _) => {
                    h125
                }
            };
            let a128 = match v1.clone() {
                Adt3::C0(_, _, h126, _, _, _, _, _) => {
                    h126
                }
            };
            if equal(a127, a128) {
                let a131 = match v0.clone() {
                    Adt3::C0(_, _, _, h129, _, _, _, _) => {
                        h129
                    }
                };
                let a132 = match v1.clone() {
                    Adt3::C0(_, _, _, h130, _, _, _, _) => {
                        h130
                    }
                };
                if equal(a131, a132) {
                    let a135 = match v0.clone() {
                        Adt3::C0(_, _, _, _, h133, _, _, _) => {
                            h133
                        }
                    };
                    let a136 = match v1.clone() {
                        Adt3::C0(_, _, _, _, h134, _, _, _) => {
                            h134
                        }
                    };
                    if equal(a135, a136) {
                        let a139 = match v0.clone() {
                            Adt3::C0(_, _, _, _, _, h137, _, _) => {
                                h137
                            }
                        };
                        let a140 = match v1.clone() {
                            Adt3::C0(_, _, _, _, _, h138, _, _) => {
                                h138
                            }
                        };
                        if equal(a139, a140) {
                            let a143 = match v0.clone() {
                                Adt3::C0(_, _, _, _, _, _, h141, _) => {
                                    h141
                                }
                            };
                            let a144 = match v1.clone() {
                                Adt3::C0(_, _, _, _, _, _, h142, _) => {
                                    h142
                                }
                            };
                            if equal(a143, a144) {
                                let a147 = match v0 {
                                    Adt3::C0(_, _, _, _, _, _, _, h145) => {
                                        h145
                                    }
                                };
                                let a148 = match v1 {
                                    Adt3::C0(_, _, _, _, _, _, _, h146) => {
                                        h146
                                    }
                                };
                                equal(a147, a148)
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    }
}

pub fn f19(v0: Adt7) -> Adt3 {
    Adt3::C0(match v0.clone() {
        Adt7::C0(h149, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h149
        }
    }, match v0.clone() {
        Adt7::C0(_, h150, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h150
        }
    }, match v0.clone() {
        Adt7::C0(_, _, h151, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h151
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, h152, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h152
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, h153, _, _, _, _, _, _, _, _, _, _, _) => {
            h153
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, h154, _, _, _, _, _, _, _, _, _, _) => {
            h154
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, h155, _, _, _, _, _, _, _, _, _) => {
            h155
        }
    }, match v0 {
        Adt7::C0(_, _, _, _, _, _, _, h156, _, _, _, _, _, _, _, _) => {
            h156
        }
    })
}

pub fn f20(v0: Bytes, v1: u64, v2: u64) -> R<Adt7> {
    Ok(Adt7::C0(f9(v0.clone(), v1, {
        let a157 = v2;
        let a158 = 0u64;
        nat_add(a157, a158)?
    })?, f9(v0.clone(), v1, {
        let a159 = v2;
        let a160 = 4u64;
        nat_add(a159, a160)?
    })?, f9(v0.clone(), v1, {
        let a161 = v2;
        let a162 = 8u64;
        nat_add(a161, a162)?
    })?, f9(v0.clone(), v1, {
        let a163 = v2;
        let a164 = 12u64;
        nat_add(a163, a164)?
    })?, f9(v0.clone(), v1, {
        let a165 = v2;
        let a166 = 16u64;
        nat_add(a165, a166)?
    })?, f9(v0.clone(), v1, {
        let a167 = v2;
        let a168 = 20u64;
        nat_add(a167, a168)?
    })?, f9(v0.clone(), v1, {
        let a169 = v2;
        let a170 = 24u64;
        nat_add(a169, a170)?
    })?, f9(v0.clone(), v1, {
        let a171 = v2;
        let a172 = 28u64;
        nat_add(a171, a172)?
    })?, f9(v0.clone(), v1, {
        let a173 = v2;
        let a174 = 32u64;
        nat_add(a173, a174)?
    })?, f9(v0.clone(), v1, {
        let a175 = v2;
        let a176 = 36u64;
        nat_add(a175, a176)?
    })?, f9(v0.clone(), v1, {
        let a177 = v2;
        let a178 = 40u64;
        nat_add(a177, a178)?
    })?, f9(v0.clone(), v1, {
        let a179 = v2;
        let a180 = 44u64;
        nat_add(a179, a180)?
    })?, f9(v0.clone(), v1, {
        let a181 = v2;
        let a182 = 48u64;
        nat_add(a181, a182)?
    })?, f9(v0.clone(), v1, {
        let a183 = v2;
        let a184 = 52u64;
        nat_add(a183, a184)?
    })?, f9(v0.clone(), v1, {
        let a185 = v2;
        let a186 = 56u64;
        nat_add(a185, a186)?
    })?, f9(v0, v1, {
        let a187 = v2;
        let a188 = 60u64;
        nat_add(a187, a188)?
    })?))
}

pub fn f21(v0: Bytes) -> R<Adt11> {
    f36(f37(f3(f4(), f38()), v0)?, 32u64)
}

pub fn f22(v0: Adt11, v1: u64) -> Adt7 {
    f39(Fn2::F40(v1), Adt7::C0(0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32), match v0 {
        Adt11::C0(_, _, _, _, _, _, h189) => {
            h189
        }
    })
}

pub fn f23() -> u32 {
    64u32
}

pub fn f24(v0: u32, v1: u32, v2: u32, v3: u32) -> u32 {
    let a194 = v0;
    let a195 = {
        let a192 = f41(v1, 8u32);
        let a193 = {
            let a190 = f41(v2, 16u32);
            let a191 = f41(v3, 24u32);
            fixed_u32::bit_or(a190, a191)
        };
        fixed_u32::bit_or(a192, a193)
    };
    fixed_u32::bit_or(a194, a195)
}

pub fn f25(v0: Bytes, v1: u64, v2: u64) -> u32 {
    let a196 = v2;
    let a197 = v1;
    if nat_lt(a196, a197) {
        let m200 = {
            let a198 = v0;
            let a199 = v2;
            index_bytes(a198, a199)
        };
        match m200 {
            None => {
                0u32
            }
            Some(v3) => {
                let m202 = {
                    let a201 = v3;
                    fixed_u32::convert(i128::from(a201))
                };
                match m202 {
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

pub fn f26(v0: u64, v1: u64) -> u32 {
    let (_, h203) = {
        let (_, h204) = f42(Fn3::F43, v1, (v0, (1u32, 0u32)));
        h204
    };
    h203
}

pub fn f27(v0: Adt3, v1: u32, v2: Bytes, v3: u64, v4: Adt9) -> R<Option<Adt9>> {
    if match v4.clone() {
        Adt9::C0(_, _, _, _, _, _, h205) => {
            h205
        }
    } {
        Ok(None::<Adt9>)
    } else {
        let a207 = match v4.clone() {
            Adt9::C0(_, h206, _, _, _, _, _) => {
                h206
            }
        };
        let a208 = 1u64;
        if nat_le(a207, a208) {
            Ok(None::<Adt9>)
        } else {
            let a213 = {
                let a210 = match v4.clone() {
                    Adt9::C0(_, _, h209, _, _, _, _) => {
                        h209
                    }
                };
                let a211 = 64u64;
                nat_add(a210, a211)?
            };
            let a214 = {
                let a212 = v2.clone();
                length_bytes(a212)
            };
            if nat_le(a213, a214) {
                let v5: Adt7 = f20(v2.clone(), {
                    let a215 = v2;
                    length_bytes(a215)
                }, match v4.clone() {
                    Adt9::C0(_, _, h216, _, _, _, _) => {
                        h216
                    }
                })?;
                let v6: u32 = {
                    let a220 = v1;
                    let a221 = if match v4.clone() {
                        Adt9::C0(_, _, _, _, h217, _, _) => {
                            h217
                        }
                    } {
                        let a218 = f44();
                        let a219 = f35();
                        fixed_u32::bit_or(a218, a219)
                    } else {
                        f44()
                    };
                    fixed_u32::bit_or(a220, a221)
                };
                let v7: Adt7 = f34(v0, v5.clone(), 0u32, 0u32, 64u32, v6)?;
                if f18(f19(v7), match v4.clone() {
                    Adt9::C0(_, _, _, h222, _, _, _) => {
                        h222
                    }
                }) {
                    let v8: u64 = f45(match v4.clone() {
                        Adt9::C0(_, h223, _, _, _, _, _) => {
                            h223
                        }
                    })?;
                    let a227 = v3;
                    let a228 = {
                        let a225 = match v4.clone() {
                            Adt9::C0(h224, _, _, _, _, _, _) => {
                                h224
                            }
                        };
                        let a226 = v8;
                        nat_add(a225, a226)?
                    };
                    if nat_lt(a227, a228) {
                        Ok(Some(Adt9::C0(match v4.clone() {
                            Adt9::C0(h229, _, _, _, _, _, _) => {
                                h229
                            }
                        }, v8, {
                            let a231 = match v4.clone() {
                                Adt9::C0(_, _, h230, _, _, _, _) => {
                                    h230
                                }
                            };
                            let a232 = 64u64;
                            nat_add(a231, a232)?
                        }, f19(v5), false, {
                            let a234 = match v4.clone() {
                                Adt9::C0(_, _, _, _, _, h233, _) => {
                                    h233
                                }
                            };
                            let a235 = 1u64;
                            nat_add(a234, a235)?
                        }, match v4 {
                            Adt9::C0(_, _, _, _, _, _, h236) => {
                                h236
                            }
                        })))
                    } else {
                        Ok(Some(Adt9::C0({
                            let a238 = match v4.clone() {
                                Adt9::C0(h237, _, _, _, _, _, _) => {
                                    h237
                                }
                            };
                            let a239 = v8;
                            nat_add(a238, a239)?
                        }, {
                            let a241 = match v4.clone() {
                                Adt9::C0(_, h240, _, _, _, _, _) => {
                                    h240
                                }
                            };
                            let a242 = v8;
                            nat_sub(a241, a242)
                        }, {
                            let a244 = match v4.clone() {
                                Adt9::C0(_, _, h243, _, _, _, _) => {
                                    h243
                                }
                            };
                            let a245 = 64u64;
                            nat_add(a244, a245)?
                        }, f46(v5), false, {
                            let a247 = match v4.clone() {
                                Adt9::C0(_, _, _, _, _, h246, _) => {
                                    h246
                                }
                            };
                            let a248 = 1u64;
                            nat_add(a247, a248)?
                        }, match v4 {
                            Adt9::C0(_, _, _, _, _, _, h249) => {
                                h249
                            }
                        })))
                    }
                } else {
                    Ok(Some(Adt9::C0(match v4.clone() {
                        Adt9::C0(h250, _, _, _, _, _, _) => {
                            h250
                        }
                    }, match v4.clone() {
                        Adt9::C0(_, h251, _, _, _, _, _) => {
                            h251
                        }
                    }, match v4.clone() {
                        Adt9::C0(_, _, h252, _, _, _, _) => {
                            h252
                        }
                    }, match v4.clone() {
                        Adt9::C0(_, _, _, h253, _, _, _) => {
                            h253
                        }
                    }, match v4.clone() {
                        Adt9::C0(_, _, _, _, h254, _, _) => {
                            h254
                        }
                    }, match v4 {
                        Adt9::C0(_, _, _, _, _, h255, _) => {
                            h255
                        }
                    }, true)))
                }
            } else {
                Ok(Some(Adt9::C0(match v4.clone() {
                    Adt9::C0(h256, _, _, _, _, _, _) => {
                        h256
                    }
                }, match v4.clone() {
                    Adt9::C0(_, h257, _, _, _, _, _) => {
                        h257
                    }
                }, match v4.clone() {
                    Adt9::C0(_, _, h258, _, _, _, _) => {
                        h258
                    }
                }, match v4.clone() {
                    Adt9::C0(_, _, _, h259, _, _, _) => {
                        h259
                    }
                }, match v4.clone() {
                    Adt9::C0(_, _, _, _, h260, _, _) => {
                        h260
                    }
                }, match v4 {
                    Adt9::C0(_, _, _, _, _, h261, _) => {
                        h261
                    }
                }, true)))
            }
        }
    }
}

pub fn f28(v0: u64) -> R<u64> {
    let a262 = v0;
    let a263 = 0u64;
    if nat_eq(a262, a263) {
        Ok(0u64)
    } else {
        let a269 = {
            let a266 = {
                let a264 = v0;
                let a265 = 1u64;
                nat_sub(a264, a265)
            };
            let a267 = 64u64;
            let a268 = 0u64;
            nat_quot(a266, a267, a268)
        };
        let a270 = 64u64;
        nat_mul(a269, a270)
    }
}

pub fn f29(v0: Fn1, p274: u64, p275: (u64, (Adt3, List<Adt5>))) -> R<(u64, (Adt3, List<Adt5>))> {
    let m273 = (p274, p275);
    {
        let mut m273 = m273;
        loop {
            let step = {
                let (v1, v2) = m273;
                let m271 = v1;
                if m271 == 0 {
                    Err::<(u64, (u64, (Adt3, List<Adt5>))), R<(u64, (Adt3, List<Adt5>))>>(Ok(v2))
                } else {
                    let v3 = m271 - 1;
                    Ok::<(u64, (u64, (Adt3, List<Adt5>))), R<(u64, (Adt3, List<Adt5>))>>((v3, {
                        let c272: Fn1 = v0.clone();
                        c272.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m273 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f30(v0: u32, v1: u64, v2: u64, v3: Adt3, v4: u64, v5: Bytes, v6: (u64, (Adt3, List<Adt5>))) -> R<(u64, (Adt3, List<Adt5>))> {
    let v7: u64 = {
        let (h276, _) = v6.clone();
        h276
    };
    let v8: Adt6 = f16(f31(v3, v0, v5, v4, v1, v2, v7, {
        let (h277, _) = {
            let (_, h278) = v6.clone();
            h278
        };
        h277
    }, 64u64, f33(v7))?)?;
    Ok(({
        let a279 = v7;
        let a280 = 64u64;
        nat_add(a279, a280)?
    }, (f19(match v8.clone() {
        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h281) => {
            h281
        }
    }), List::cons(Adt5::C0(v8), {
        let (_, h282) = {
            let (_, h283) = v6;
            h283
        };
        h282
    }))))
}

pub fn f31(_: Adt3, v1: u32, v2: Bytes, v3: u64, v4: u64, v5: u64, v6: u64, v7: Adt3, v8: u64, v9: u32) -> R<Adt10> {
    Ok(Adt10::C0(0u64, v5, 1u64, {
        let a284 = v6;
        let a285 = 64u64;
        let a286 = 0u64;
        nat_quot(a284, a285, a286)
    }, {
        let a289 = {
            let a287 = v5;
            let a288 = 1024u64;
            nat_mul(a287, a288)?
        };
        let a290 = v6;
        nat_add(a289, a290)?
    }, v7, f20(v2, v3, {
        let a291 = v4;
        let a292 = v6;
        nat_add(a291, a292)?
    })?, v5, f26(v8, 7u64), {
        let a293 = v1;
        let a294 = v9;
        fixed_u32::bit_or(a293, a294)
    }))
}

pub fn f32() -> u32 {
    2u32
}

pub fn f33(v0: u64) -> u32 {
    let a295 = v0;
    let a296 = 0u64;
    if nat_eq(a295, a296) {
        f47()
    } else {
        0u32
    }
}

pub fn f34(v0: Adt3, v1: Adt7, v2: u32, v3: u32, v4: u32, v5: u32) -> R<Adt7> {
    Ok(f48(f49(v0, v1, v2, v3, v4, v5)?))
}

pub fn f35() -> u32 {
    8u32
}

pub fn f36(v0: Adt2, v1: u64) -> R<Adt11> {
    let v2: Adt3 = match v0.clone() {
        Adt2::C0(h297, _, _, _, _, _, _) => {
            h297
        }
    };
    let v3: u32 = match v0.clone() {
        Adt2::C0(_, h298, _, _, _, _, _) => {
            h298
        }
    };
    let v4: Bytes = match v0.clone() {
        Adt2::C0(_, _, _, h299, _, _, _) => {
            h299
        }
    };
    let v5: (Adt10, List<Adt5>) = f15(v2.clone(), v3, v4.clone(), 0u64, {
        let a300 = v4.clone();
        length_bytes(a300)
    }, match v0.clone() {
        Adt2::C0(_, _, h301, _, _, _, _) => {
            h301
        }
    }, match v0.clone() {
        Adt2::C0(_, _, _, _, _, h302, _) => {
            h302
        }
    })?;
    let v6: (Adt10, (List<Adt5>, u64)) = f50(Fn4::F51(v3, v2.clone()), ({
        let (h303, _) = v5;
        h303
    }, ({
        let (_, h304) = v5;
        h304
    }, {
        let a306 = match v0.clone() {
            Adt2::C0(_, _, _, _, h305, _, _) => {
                h305
            }
        };
        length_list(a306)
    })), match v0.clone() {
        Adt2::C0(_, _, _, _, h307, _, _) => {
            h307
        }
    })?;
    let v7: (u64, List<Adt5>) = f52(Fn5::F53(v6.clone()), {
        let a310 = {
            let a308 = v1;
            let a309 = 63u64;
            nat_add(a308, a309)?
        };
        let a311 = 64u64;
        let a312 = 0u64;
        nat_quot(a310, a311, a312)
    }, (0u64, {
        let (h313, _) = {
            let (_, h314) = v6;
            h314
        };
        h313
    }))?;
    Ok(Adt11::C0(f54(v3), {
        let a319 = {
            let a316 = match v0.clone() {
                Adt2::C0(_, _, h315, _, _, _, _) => {
                    h315
                }
            };
            let a317 = 1024u64;
            nat_mul(a316, a317)?
        };
        let a320 = {
            let a318 = v4;
            length_bytes(a318)
        };
        nat_add(a319, a320)?
    }, v1, v2, v3, match v0 {
        Adt2::C0(_, _, _, _, _, _, h321) => {
            h321
        }
    }, f55({
        let (_, h322) = v7;
        h322
    })))
}

pub fn f37(v0: Adt2, v1: Bytes) -> R<Adt2> {
    let v2: Bytes = {
        let a324 = match v0.clone() {
            Adt2::C0(_, _, _, h323, _, _, _) => {
                h323
            }
        };
        let a325 = v1;
        append_bytes(a324, a325)
    };
    let v3: u64 = {
        let a326 = v2.clone();
        length_bytes(a326)
    };
    let v4: u64 = {
        let a327 = v3;
        let a328 = 0u64;
        if nat_eq(a327, a328) {
            0u64
        } else {
            let a331 = {
                let a329 = v3;
                let a330 = 1u64;
                nat_sub(a329, a330)
            };
            let a332 = 1024u64;
            let a333 = 0u64;
            nat_quot(a331, a332, a333)
        }
    };
    let v5: u32 = match v0.clone() {
        Adt2::C0(_, h334, _, _, _, _, _) => {
            h334
        }
    };
    let v6: Adt3 = match v0.clone() {
        Adt2::C0(h335, _, _, _, _, _, _) => {
            h335
        }
    };
    let v7: (u64, (List<Adt4>, List<Adt5>)) = f56(Fn6::F57(v5, v2.clone(), v6.clone(), v0.clone()), v4, (0u64, (match v0.clone() {
        Adt2::C0(_, _, _, _, h336, _, _) => {
            h336
        }
    }, match v0.clone() {
        Adt2::C0(_, _, _, _, _, h337, _) => {
            h337
        }
    })))?;
    let v8: u64 = {
        let a338 = v4;
        let a339 = 1024u64;
        nat_mul(a338, a339)?
    };
    Ok(Adt2::C0(v6, v5, {
        let a341 = match v0.clone() {
            Adt2::C0(_, _, h340, _, _, _, _) => {
                h340
            }
        };
        let a342 = v4;
        nat_add(a341, a342)?
    }, {
        let m348 = {
            let a345 = v2;
            let a346 = v8;
            let a347 = {
                let a343 = v3;
                let a344 = v8;
                nat_sub(a343, a344)
            };
            slice_bytes(a345, a346, a347)
        };
        match m348 {
            None => {
                Bytes::lit(&[])
            }
            Some(v9) => {
                v9
            }
        }
    }, {
        let (h349, _) = {
            let (_, h350) = v7.clone();
            h350
        };
        h349
    }, {
        let (_, h351) = {
            let (_, h352) = v7;
            h352
        };
        h351
    }, match v0 {
        Adt2::C0(_, _, _, _, _, _, h353) => {
            h353
        }
    }))
}

pub fn f38() -> u32 {
    32u32
}

pub fn f39(v0: Fn2, p357: Adt7, p358: List<Adt5>) -> Adt7 {
    let m356 = (p357, p358);
    {
        let mut m356 = m356;
        loop {
            let step = {
                let (v1, v2) = m356;
                let m354 = v2;
                match m354.uncons() {
                    None => {
                        Err::<(Adt7, List<Adt5>), Adt7>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt7, List<Adt5>), Adt7>(({
                            let c355: Fn2 = v0.clone();
                            c355.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m356 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f40(v0: u64, v1: Adt7, v2: Adt5) -> Adt7 {
    let m359 = v2;
    match m359 {
        Adt5::C0(v3) => {
            let a361 = match v3.clone() {
                Adt6::C0(h360, _, _, _, _, _, _, _, _, _, _, _) => {
                    h360
                }
            };
            let a362 = 2u64;
            if nat_eq(a361, a362) {
                let a364 = match v3.clone() {
                    Adt6::C0(_, _, _, h363, _, _, _, _, _, _, _, _) => {
                        h363
                    }
                };
                let a365 = v0;
                if nat_eq(a364, a365) {
                    match v3 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h366) => {
                            h366
                        }
                    }
                } else {
                    v1
                }
            } else {
                v1
            }
        }
        Adt5::C1(_, _) => {
            v1
        }
        Adt5::C2(_, _) => {
            v1
        }
    }
}

pub fn f41(v0: u32, v1: u32) -> u32 {
    let m369 = {
        let a367 = v0;
        let a368 = v1;
        fixed_u32::shift_left(a367, a368)
    };
    match m369 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f42(v0: Fn3, p373: u64, p374: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let m372 = (p373, p374);
    {
        let mut m372 = m372;
        loop {
            let step = {
                let (v1, v2) = m372;
                let m370 = v1;
                if m370 == 0 {
                    Err::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>(v2)
                } else {
                    let v3 = m370 - 1;
                    Ok::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>((v3, {
                        let c371: Fn3 = v0.clone();
                        c371.apply(v2)
                    }))
                }
            };
            match step {
                Ok(next) => { m372 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f43(v0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let v1: u64 = {
        let (h375, _) = v0;
        h375
    };
    let v2: u32 = {
        let (h376, _) = {
            let (_, h377) = v0;
            h377
        };
        h376
    };
    let v3: u32 = {
        let (_, h378) = {
            let (_, h379) = v0;
            h379
        };
        h378
    };
    ({
        let a380 = v1;
        let a381 = 2u64;
        let a382 = 0u64;
        nat_quot(a380, a381, a382)
    }, (f41(v2, 1u32), {
        let a386 = {
            let a383 = v1;
            let a384 = 2u64;
            let a385 = 0u64;
            nat_rem(a383, a384, a385)
        };
        let a387 = 1u64;
        if nat_eq(a386, a387) {
            let a388 = v3;
            let a389 = v2;
            fixed_u32::bit_or(a388, a389)
        } else {
            v3
        }
    }))
}

pub fn f44() -> u32 {
    4u32
}

pub fn f45(v0: u64) -> R<u64> {
    let (h390, _) = f58(Fn7::F59(v0), 64u64, 1u64)?;
    Ok(h390)
}

pub fn f46(v0: Adt7) -> Adt3 {
    Adt3::C0(match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, _, h391, _, _, _, _, _, _, _) => {
            h391
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, _, _, h392, _, _, _, _, _, _) => {
            h392
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, _, _, _, h393, _, _, _, _, _) => {
            h393
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, h394, _, _, _, _) => {
            h394
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, h395, _, _, _) => {
            h395
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h396, _, _) => {
            h396
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h397, _) => {
            h397
        }
    }, match v0 {
        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h398) => {
            h398
        }
    })
}

pub fn f47() -> u32 {
    1u32
}

pub fn f48(v0: Adt12) -> Adt7 {
    f60(Fn8::F61, Adt7::C0(0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32), match v0 {
        Adt12::C0(_, _, _, _, _, _, h399) => {
            h399
        }
    })
}

pub fn f49(v0: Adt3, v1: Adt7, v2: u32, v3: u32, v4: u32, v5: u32) -> R<Adt12> {
    let v6: (u64, Adt15) = f62(Fn9::F63(v0.clone(), v1.clone()), 800u64, (0u64, Adt15::C0(f64(v0.clone(), v2, v3, v4, v5), Adt7::C0(0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32), List::<Adt13>::nil())))?;
    Ok(Adt12::C0(v0, v1, v2, v3, v4, v5, {
        let m402 = {
            let (_, h401) = v6;
            h401
        };
        match m402 {
            Adt15::C0(_, _, h400) => {
                h400
            }
        }
    }))
}

pub fn f50(v0: Fn4, p406: (Adt10, (List<Adt5>, u64)), p407: List<Adt4>) -> R<(Adt10, (List<Adt5>, u64))> {
    let m405 = (p406, p407);
    {
        let mut m405 = m405;
        loop {
            let step = {
                let (v1, v2) = m405;
                let m403 = v2;
                match m403.uncons() {
                    None => {
                        Err::<((Adt10, (List<Adt5>, u64)), List<Adt4>), R<(Adt10, (List<Adt5>, u64))>>(Ok(v1))
                    }
                    Some((v3, v4)) => {
                        Ok::<((Adt10, (List<Adt5>, u64)), List<Adt4>), R<(Adt10, (List<Adt5>, u64))>>(({
                            let c404: Fn4 = v0.clone();
                            c404.apply(v1, v3)?
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m405 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f51(v0: u32, v1: Adt3, v2: (Adt10, (List<Adt5>, u64)), v3: Adt4) -> R<(Adt10, (List<Adt5>, u64))> {
    let v4: Adt6 = f16({
        let (h408, _) = v2.clone();
        h408
    })?;
    Ok((f65(v3.clone(), Adt4::C0(f19(match v4.clone() {
        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h409) => {
            h409
        }
    }), {
        let m412 = {
            let (h411, _) = v2.clone();
            h411
        };
        match m412 {
            Adt10::C0(_, h410, _, _, _, _, _, _, _, _) => {
                h410
            }
        }
    }, {
        let m415 = {
            let (h414, _) = v2.clone();
            h414
        };
        match m415 {
            Adt10::C0(_, _, h413, _, _, _, _, _, _, _) => {
                h413
            }
        }
    }), v1, v0)?, (List::cons(Adt5::C2(v3, {
        let (_, h416) = {
            let (_, h417) = v2.clone();
            h417
        };
        h416
    }), List::cons(Adt5::C0(v4), {
        let (h418, _) = {
            let (_, h419) = v2.clone();
            h419
        };
        h418
    })), {
        let a422 = {
            let (_, h420) = {
                let (_, h421) = v2;
                h421
            };
            h420
        };
        let a423 = 1u64;
        nat_sub(a422, a423)
    })))
}

pub fn f52(v0: Fn5, p427: u64, p428: (u64, List<Adt5>)) -> R<(u64, List<Adt5>)> {
    let m426 = (p427, p428);
    {
        let mut m426 = m426;
        loop {
            let step = {
                let (v1, v2) = m426;
                let m424 = v1;
                if m424 == 0 {
                    Err::<(u64, (u64, List<Adt5>)), R<(u64, List<Adt5>)>>(Ok(v2))
                } else {
                    let v3 = m424 - 1;
                    Ok::<(u64, (u64, List<Adt5>)), R<(u64, List<Adt5>)>>((v3, {
                        let c425: Fn5 = v0.clone();
                        c425.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m426 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f53(v0: (Adt10, (List<Adt5>, u64)), v1: (u64, List<Adt5>)) -> R<(u64, List<Adt5>)> {
    Ok(({
        let a430 = {
            let (h429, _) = v1.clone();
            h429
        };
        let a431 = 1u64;
        nat_add(a430, a431)?
    }, List::cons(Adt5::C0(f16(f17({
        let (h432, _) = v0;
        h432
    }, {
        let (h433, _) = v1;
        h433
    }))?), {
        let (_, h434) = v1;
        h434
    })))
}

pub fn f54(v0: u32) -> u64 {
    let a435 = v0;
    let a436 = f6();
    if equal(a435, a436) {
        1u64
    } else {
        let a437 = v0;
        let a438 = f23();
        if equal(a437, a438) {
            2u64
        } else {
            let a439 = v0;
            let a440 = f38();
            if equal(a439, a440) {
                3u64
            } else {
                0u64
            }
        }
    }
}

pub fn f55(v0: List<Adt5>) -> List<Adt5> {
    f66(Fn10::F67, List::<Adt5>::nil(), v0)
}

pub fn f56(v0: Fn6, p444: u64, p445: (u64, (List<Adt4>, List<Adt5>))) -> R<(u64, (List<Adt4>, List<Adt5>))> {
    let m443 = (p444, p445);
    {
        let mut m443 = m443;
        loop {
            let step = {
                let (v1, v2) = m443;
                let m441 = v1;
                if m441 == 0 {
                    Err::<(u64, (u64, (List<Adt4>, List<Adt5>))), R<(u64, (List<Adt4>, List<Adt5>))>>(Ok(v2))
                } else {
                    let v3 = m441 - 1;
                    Ok::<(u64, (u64, (List<Adt4>, List<Adt5>))), R<(u64, (List<Adt4>, List<Adt5>))>>((v3, {
                        let c442: Fn6 = v0.clone();
                        c442.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m443 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f57(v0: u32, v1: Bytes, v2: Adt3, v3: Adt2, v4: (u64, (List<Adt4>, List<Adt5>))) -> R<(u64, (List<Adt4>, List<Adt5>))> {
    let v5: u64 = {
        let a448 = match v3 {
            Adt2::C0(_, _, h446, _, _, _, _) => {
                h446
            }
        };
        let a449 = {
            let (h447, _) = v4.clone();
            h447
        };
        nat_add(a448, a449)?
    };
    let v6: (Adt10, List<Adt5>) = f15(v2.clone(), v0, v1, {
        let a451 = {
            let (h450, _) = v4.clone();
            h450
        };
        let a452 = 1024u64;
        nat_mul(a451, a452)?
    }, 1024u64, v5, {
        let (_, h453) = {
            let (_, h454) = v4.clone();
            h454
        };
        h453
    })?;
    let v7: Adt6 = f16({
        let (h455, _) = v6;
        h455
    })?;
    Ok(({
        let a457 = {
            let (h456, _) = v4;
            h456
        };
        let a458 = 1u64;
        nat_add(a457, a458)?
    }, f68(v2, v0, Adt16::C0(Adt4::C0(f19(match v7.clone() {
        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h459) => {
            h459
        }
    }), v5, 1u64), {
        let a460 = v5;
        let a461 = 1u64;
        nat_add(a460, a461)?
    }, {
        let (h462, _) = {
            let (_, h463) = v4;
            h463
        };
        h462
    }, List::cons(Adt5::C0(v7), {
        let (_, h464) = v6;
        h464
    })))?))
}

pub fn f58(v0: Fn7, p469: u64, p470: u64) -> R<(u64, bool)> {
    let m468 = (p469, p470);
    {
        let mut m468 = m468;
        loop {
            let step = {
                let (v1, v2) = m468;
                let m465 = v1;
                if m465 == 0 {
                    Err::<(u64, u64), R<(u64, bool)>>(Ok((v2, false)))
                } else {
                    let v3 = m465 - 1;
                    let m467 = {
                        let c466: Fn7 = v0.clone();
                        c466.apply(v2)?
                    };
                    match m467 {
                        None => {
                            Err::<(u64, u64), R<(u64, bool)>>(Ok((v2, true)))
                        }
                        Some(v4) => {
                            Ok::<(u64, u64), R<(u64, bool)>>((v3, v4))
                        }
                    }
                }
            };
            match step {
                Ok(next) => { m468 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f59(v0: u64, v1: u64) -> R<Option<u64>> {
    let a473 = {
        let a471 = v1;
        let a472 = 2u64;
        nat_mul(a471, a472)?
    };
    let a474 = v0;
    if nat_lt(a473, a474) {
        Ok(Some({
            let a475 = v1;
            let a476 = 2u64;
            nat_mul(a475, a476)?
        }))
    } else {
        Ok(None::<u64>)
    }
}

pub fn f60(v0: Fn8, p480: Adt7, p481: List<Adt13>) -> Adt7 {
    let m479 = (p480, p481);
    {
        let mut m479 = m479;
        loop {
            let step = {
                let (v1, v2) = m479;
                let m477 = v2;
                match m477.uncons() {
                    None => {
                        Err::<(Adt7, List<Adt13>), Adt7>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt7, List<Adt13>), Adt7>(({
                            let c478: Fn8 = v0.clone();
                            c478.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m479 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f61(v0: Adt7, v1: Adt13) -> Adt7 {
    let a483 = match v1.clone() {
        Adt13::C0(h482, _, _, _, _, _, _, _, _) => {
            h482
        }
    };
    let a484 = 7u64;
    if nat_eq(a483, a484) {
        f69(v0, match v1.clone() {
            Adt13::C0(_, _, _, _, h485, _, _, _, _) => {
                h485
            }
        }, match v1 {
            Adt13::C0(_, _, _, _, _, _, _, _, h486) => {
                h486
            }
        })
    } else {
        v0
    }
}

pub fn f62(v0: Fn9, p490: u64, p491: (u64, Adt15)) -> R<(u64, Adt15)> {
    let m489 = (p490, p491);
    {
        let mut m489 = m489;
        loop {
            let step = {
                let (v1, v2) = m489;
                let m487 = v1;
                if m487 == 0 {
                    Err::<(u64, (u64, Adt15)), R<(u64, Adt15)>>(Ok(v2))
                } else {
                    let v3 = m487 - 1;
                    Ok::<(u64, (u64, Adt15)), R<(u64, Adt15)>>((v3, {
                        let c488: Fn9 = v0.clone();
                        c488.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m489 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f63(v0: Adt3, v1: Adt7, v2: (u64, Adt15)) -> R<(u64, Adt15)> {
    Ok(({
        let a493 = {
            let (h492, _) = v2.clone();
            h492
        };
        let a494 = 1u64;
        nat_add(a493, a494)?
    }, f70(v0, v1, {
        let (_, h495) = v2;
        h495
    }, {
        let (h496, _) = v2;
        h496
    })?))
}

pub fn f64(v0: Adt3, v1: u32, v2: u32, v3: u32, v4: u32) -> Adt7 {
    Adt7::C0(match v0.clone() {
        Adt3::C0(h497, _, _, _, _, _, _, _) => {
            h497
        }
    }, match v0.clone() {
        Adt3::C0(_, h498, _, _, _, _, _, _) => {
            h498
        }
    }, match v0.clone() {
        Adt3::C0(_, _, h499, _, _, _, _, _) => {
            h499
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h500, _, _, _, _) => {
            h500
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h501, _, _, _) => {
            h501
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h502, _, _) => {
            h502
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h503, _) => {
            h503
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, h504) => {
            h504
        }
    }, 1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, v1, v2, v3, v4)
}

pub fn f65(v0: Adt4, v1: Adt4, v2: Adt3, v3: u32) -> R<Adt10> {
    Ok(Adt10::C0(1u64, match v0.clone() {
        Adt4::C0(_, h505, _) => {
            h505
        }
    }, {
        let a508 = match v0.clone() {
            Adt4::C0(_, _, h506) => {
                h506
            }
        };
        let a509 = match v1.clone() {
            Adt4::C0(_, _, h507) => {
                h507
            }
        };
        nat_add(a508, a509)?
    }, 0u64, 0u64, v2, Adt7::C0({
        let m512 = match v0.clone() {
            Adt4::C0(h511, _, _) => {
                h511
            }
        };
        match m512 {
            Adt3::C0(h510, _, _, _, _, _, _, _) => {
                h510
            }
        }
    }, {
        let m515 = match v0.clone() {
            Adt4::C0(h514, _, _) => {
                h514
            }
        };
        match m515 {
            Adt3::C0(_, h513, _, _, _, _, _, _) => {
                h513
            }
        }
    }, {
        let m518 = match v0.clone() {
            Adt4::C0(h517, _, _) => {
                h517
            }
        };
        match m518 {
            Adt3::C0(_, _, h516, _, _, _, _, _) => {
                h516
            }
        }
    }, {
        let m521 = match v0.clone() {
            Adt4::C0(h520, _, _) => {
                h520
            }
        };
        match m521 {
            Adt3::C0(_, _, _, h519, _, _, _, _) => {
                h519
            }
        }
    }, {
        let m524 = match v0.clone() {
            Adt4::C0(h523, _, _) => {
                h523
            }
        };
        match m524 {
            Adt3::C0(_, _, _, _, h522, _, _, _) => {
                h522
            }
        }
    }, {
        let m527 = match v0.clone() {
            Adt4::C0(h526, _, _) => {
                h526
            }
        };
        match m527 {
            Adt3::C0(_, _, _, _, _, h525, _, _) => {
                h525
            }
        }
    }, {
        let m530 = match v0.clone() {
            Adt4::C0(h529, _, _) => {
                h529
            }
        };
        match m530 {
            Adt3::C0(_, _, _, _, _, _, h528, _) => {
                h528
            }
        }
    }, {
        let m533 = match v0 {
            Adt4::C0(h532, _, _) => {
                h532
            }
        };
        match m533 {
            Adt3::C0(_, _, _, _, _, _, _, h531) => {
                h531
            }
        }
    }, {
        let m536 = match v1.clone() {
            Adt4::C0(h535, _, _) => {
                h535
            }
        };
        match m536 {
            Adt3::C0(h534, _, _, _, _, _, _, _) => {
                h534
            }
        }
    }, {
        let m539 = match v1.clone() {
            Adt4::C0(h538, _, _) => {
                h538
            }
        };
        match m539 {
            Adt3::C0(_, h537, _, _, _, _, _, _) => {
                h537
            }
        }
    }, {
        let m542 = match v1.clone() {
            Adt4::C0(h541, _, _) => {
                h541
            }
        };
        match m542 {
            Adt3::C0(_, _, h540, _, _, _, _, _) => {
                h540
            }
        }
    }, {
        let m545 = match v1.clone() {
            Adt4::C0(h544, _, _) => {
                h544
            }
        };
        match m545 {
            Adt3::C0(_, _, _, h543, _, _, _, _) => {
                h543
            }
        }
    }, {
        let m548 = match v1.clone() {
            Adt4::C0(h547, _, _) => {
                h547
            }
        };
        match m548 {
            Adt3::C0(_, _, _, _, h546, _, _, _) => {
                h546
            }
        }
    }, {
        let m551 = match v1.clone() {
            Adt4::C0(h550, _, _) => {
                h550
            }
        };
        match m551 {
            Adt3::C0(_, _, _, _, _, h549, _, _) => {
                h549
            }
        }
    }, {
        let m554 = match v1.clone() {
            Adt4::C0(h553, _, _) => {
                h553
            }
        };
        match m554 {
            Adt3::C0(_, _, _, _, _, _, h552, _) => {
                h552
            }
        }
    }, {
        let m557 = match v1 {
            Adt4::C0(h556, _, _) => {
                h556
            }
        };
        match m557 {
            Adt3::C0(_, _, _, _, _, _, _, h555) => {
                h555
            }
        }
    }), 0u64, 64u32, {
        let a558 = v3;
        let a559 = f44();
        fixed_u32::bit_or(a558, a559)
    }))
}

pub fn f66(v0: Fn10, p563: List<Adt5>, p564: List<Adt5>) -> List<Adt5> {
    let m562 = (p563, p564);
    {
        let mut m562 = m562;
        loop {
            let step = {
                let (v1, v2) = m562;
                let m560 = v2;
                match m560.uncons() {
                    None => {
                        Err::<(List<Adt5>, List<Adt5>), List<Adt5>>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(List<Adt5>, List<Adt5>), List<Adt5>>(({
                            let c561: Fn10 = v0.clone();
                            c561.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m562 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f67(v0: List<Adt5>, v1: Adt5) -> List<Adt5> {
    List::cons(v1, v0)
}

pub fn f68(v0: Adt3, v1: u32, v2: Adt16) -> R<(List<Adt4>, List<Adt5>)> {
    let v3: Adt16 = {
        let (h565, _) = f71(Fn11::F72(v1, v0), 64u64, v2)?;
        h565
    };
    let v4: List<Adt4> = List::cons(match v3.clone() {
        Adt16::C0(h566, _, _, _) => {
            h566
        }
    }, match v3.clone() {
        Adt16::C0(_, _, h567, _) => {
            h567
        }
    });
    Ok((v4.clone(), List::cons(Adt5::C1(match v3.clone() {
        Adt16::C0(h568, _, _, _) => {
            h568
        }
    }, {
        let a569 = v4;
        length_list(a569)
    }), match v3 {
        Adt16::C0(_, _, _, h570) => {
            h570
        }
    })))
}

pub fn f69(v0: Adt7, v1: u64, v2: u32) -> Adt7 {
    let a571 = v1;
    let a572 = 8u64;
    if nat_lt(a571, a572) {
        let a573 = v1;
        let a574 = 4u64;
        if nat_lt(a573, a574) {
            let a575 = v1;
            let a576 = 2u64;
            if nat_lt(a575, a576) {
                let a577 = v1;
                let a578 = 1u64;
                if nat_lt(a577, a578) {
                    Adt7::C0(v2, match v0.clone() {
                        Adt7::C0(_, h579, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h579
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, h580, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h580
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, h581, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h581
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, h582, _, _, _, _, _, _, _, _, _, _, _) => {
                            h582
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, h583, _, _, _, _, _, _, _, _, _, _) => {
                            h583
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, h584, _, _, _, _, _, _, _, _, _) => {
                            h584
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, h585, _, _, _, _, _, _, _, _) => {
                            h585
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, h586, _, _, _, _, _, _, _) => {
                            h586
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, h587, _, _, _, _, _, _) => {
                            h587
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, h588, _, _, _, _, _) => {
                            h588
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, h589, _, _, _, _) => {
                            h589
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, h590, _, _, _) => {
                            h590
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h591, _, _) => {
                            h591
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h592, _) => {
                            h592
                        }
                    }, match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h593) => {
                            h593
                        }
                    })
                } else {
                    Adt7::C0(match v0.clone() {
                        Adt7::C0(h594, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h594
                        }
                    }, v2, match v0.clone() {
                        Adt7::C0(_, _, h595, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h595
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, h596, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h596
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, h597, _, _, _, _, _, _, _, _, _, _, _) => {
                            h597
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, h598, _, _, _, _, _, _, _, _, _, _) => {
                            h598
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, h599, _, _, _, _, _, _, _, _, _) => {
                            h599
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, h600, _, _, _, _, _, _, _, _) => {
                            h600
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, h601, _, _, _, _, _, _, _) => {
                            h601
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, h602, _, _, _, _, _, _) => {
                            h602
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, h603, _, _, _, _, _) => {
                            h603
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, h604, _, _, _, _) => {
                            h604
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, h605, _, _, _) => {
                            h605
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h606, _, _) => {
                            h606
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h607, _) => {
                            h607
                        }
                    }, match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h608) => {
                            h608
                        }
                    })
                }
            } else {
                let a609 = v1;
                let a610 = 3u64;
                if nat_lt(a609, a610) {
                    Adt7::C0(match v0.clone() {
                        Adt7::C0(h611, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h611
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, h612, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h612
                        }
                    }, v2, match v0.clone() {
                        Adt7::C0(_, _, _, h613, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h613
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, h614, _, _, _, _, _, _, _, _, _, _, _) => {
                            h614
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, h615, _, _, _, _, _, _, _, _, _, _) => {
                            h615
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, h616, _, _, _, _, _, _, _, _, _) => {
                            h616
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, h617, _, _, _, _, _, _, _, _) => {
                            h617
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, h618, _, _, _, _, _, _, _) => {
                            h618
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, h619, _, _, _, _, _, _) => {
                            h619
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, h620, _, _, _, _, _) => {
                            h620
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, h621, _, _, _, _) => {
                            h621
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, h622, _, _, _) => {
                            h622
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h623, _, _) => {
                            h623
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h624, _) => {
                            h624
                        }
                    }, match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h625) => {
                            h625
                        }
                    })
                } else {
                    Adt7::C0(match v0.clone() {
                        Adt7::C0(h626, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h626
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, h627, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h627
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, h628, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h628
                        }
                    }, v2, match v0.clone() {
                        Adt7::C0(_, _, _, _, h629, _, _, _, _, _, _, _, _, _, _, _) => {
                            h629
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, h630, _, _, _, _, _, _, _, _, _, _) => {
                            h630
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, h631, _, _, _, _, _, _, _, _, _) => {
                            h631
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, h632, _, _, _, _, _, _, _, _) => {
                            h632
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, h633, _, _, _, _, _, _, _) => {
                            h633
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, h634, _, _, _, _, _, _) => {
                            h634
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, h635, _, _, _, _, _) => {
                            h635
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, h636, _, _, _, _) => {
                            h636
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, h637, _, _, _) => {
                            h637
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h638, _, _) => {
                            h638
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h639, _) => {
                            h639
                        }
                    }, match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h640) => {
                            h640
                        }
                    })
                }
            }
        } else {
            let a641 = v1;
            let a642 = 6u64;
            if nat_lt(a641, a642) {
                let a643 = v1;
                let a644 = 5u64;
                if nat_lt(a643, a644) {
                    Adt7::C0(match v0.clone() {
                        Adt7::C0(h645, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h645
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, h646, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h646
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, h647, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h647
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, h648, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h648
                        }
                    }, v2, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, h649, _, _, _, _, _, _, _, _, _, _) => {
                            h649
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, h650, _, _, _, _, _, _, _, _, _) => {
                            h650
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, h651, _, _, _, _, _, _, _, _) => {
                            h651
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, h652, _, _, _, _, _, _, _) => {
                            h652
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, h653, _, _, _, _, _, _) => {
                            h653
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, h654, _, _, _, _, _) => {
                            h654
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, h655, _, _, _, _) => {
                            h655
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, h656, _, _, _) => {
                            h656
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h657, _, _) => {
                            h657
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h658, _) => {
                            h658
                        }
                    }, match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h659) => {
                            h659
                        }
                    })
                } else {
                    Adt7::C0(match v0.clone() {
                        Adt7::C0(h660, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h660
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, h661, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h661
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, h662, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h662
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, h663, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h663
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, h664, _, _, _, _, _, _, _, _, _, _, _) => {
                            h664
                        }
                    }, v2, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, h665, _, _, _, _, _, _, _, _, _) => {
                            h665
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, h666, _, _, _, _, _, _, _, _) => {
                            h666
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, h667, _, _, _, _, _, _, _) => {
                            h667
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, h668, _, _, _, _, _, _) => {
                            h668
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, h669, _, _, _, _, _) => {
                            h669
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, h670, _, _, _, _) => {
                            h670
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, h671, _, _, _) => {
                            h671
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h672, _, _) => {
                            h672
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h673, _) => {
                            h673
                        }
                    }, match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h674) => {
                            h674
                        }
                    })
                }
            } else {
                let a675 = v1;
                let a676 = 7u64;
                if nat_lt(a675, a676) {
                    Adt7::C0(match v0.clone() {
                        Adt7::C0(h677, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h677
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, h678, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h678
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, h679, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h679
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, h680, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h680
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, h681, _, _, _, _, _, _, _, _, _, _, _) => {
                            h681
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, h682, _, _, _, _, _, _, _, _, _, _) => {
                            h682
                        }
                    }, v2, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, h683, _, _, _, _, _, _, _, _) => {
                            h683
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, h684, _, _, _, _, _, _, _) => {
                            h684
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, h685, _, _, _, _, _, _) => {
                            h685
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, h686, _, _, _, _, _) => {
                            h686
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, h687, _, _, _, _) => {
                            h687
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, h688, _, _, _) => {
                            h688
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h689, _, _) => {
                            h689
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h690, _) => {
                            h690
                        }
                    }, match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h691) => {
                            h691
                        }
                    })
                } else {
                    Adt7::C0(match v0.clone() {
                        Adt7::C0(h692, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h692
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, h693, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h693
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, h694, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h694
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, h695, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h695
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, h696, _, _, _, _, _, _, _, _, _, _, _) => {
                            h696
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, h697, _, _, _, _, _, _, _, _, _, _) => {
                            h697
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, h698, _, _, _, _, _, _, _, _, _) => {
                            h698
                        }
                    }, v2, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, h699, _, _, _, _, _, _, _) => {
                            h699
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, h700, _, _, _, _, _, _) => {
                            h700
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, h701, _, _, _, _, _) => {
                            h701
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, h702, _, _, _, _) => {
                            h702
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, h703, _, _, _) => {
                            h703
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h704, _, _) => {
                            h704
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h705, _) => {
                            h705
                        }
                    }, match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h706) => {
                            h706
                        }
                    })
                }
            }
        }
    } else {
        let a707 = v1;
        let a708 = 12u64;
        if nat_lt(a707, a708) {
            let a709 = v1;
            let a710 = 10u64;
            if nat_lt(a709, a710) {
                let a711 = v1;
                let a712 = 9u64;
                if nat_lt(a711, a712) {
                    Adt7::C0(match v0.clone() {
                        Adt7::C0(h713, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h713
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, h714, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h714
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, h715, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h715
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, h716, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h716
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, h717, _, _, _, _, _, _, _, _, _, _, _) => {
                            h717
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, h718, _, _, _, _, _, _, _, _, _, _) => {
                            h718
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, h719, _, _, _, _, _, _, _, _, _) => {
                            h719
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, h720, _, _, _, _, _, _, _, _) => {
                            h720
                        }
                    }, v2, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, h721, _, _, _, _, _, _) => {
                            h721
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, h722, _, _, _, _, _) => {
                            h722
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, h723, _, _, _, _) => {
                            h723
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, h724, _, _, _) => {
                            h724
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h725, _, _) => {
                            h725
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h726, _) => {
                            h726
                        }
                    }, match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h727) => {
                            h727
                        }
                    })
                } else {
                    Adt7::C0(match v0.clone() {
                        Adt7::C0(h728, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h728
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, h729, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h729
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, h730, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h730
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, h731, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h731
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, h732, _, _, _, _, _, _, _, _, _, _, _) => {
                            h732
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, h733, _, _, _, _, _, _, _, _, _, _) => {
                            h733
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, h734, _, _, _, _, _, _, _, _, _) => {
                            h734
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, h735, _, _, _, _, _, _, _, _) => {
                            h735
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, h736, _, _, _, _, _, _, _) => {
                            h736
                        }
                    }, v2, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, h737, _, _, _, _, _) => {
                            h737
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, h738, _, _, _, _) => {
                            h738
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, h739, _, _, _) => {
                            h739
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h740, _, _) => {
                            h740
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h741, _) => {
                            h741
                        }
                    }, match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h742) => {
                            h742
                        }
                    })
                }
            } else {
                let a743 = v1;
                let a744 = 11u64;
                if nat_lt(a743, a744) {
                    Adt7::C0(match v0.clone() {
                        Adt7::C0(h745, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h745
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, h746, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h746
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, h747, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h747
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, h748, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h748
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, h749, _, _, _, _, _, _, _, _, _, _, _) => {
                            h749
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, h750, _, _, _, _, _, _, _, _, _, _) => {
                            h750
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, h751, _, _, _, _, _, _, _, _, _) => {
                            h751
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, h752, _, _, _, _, _, _, _, _) => {
                            h752
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, h753, _, _, _, _, _, _, _) => {
                            h753
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, h754, _, _, _, _, _, _) => {
                            h754
                        }
                    }, v2, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, h755, _, _, _, _) => {
                            h755
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, h756, _, _, _) => {
                            h756
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h757, _, _) => {
                            h757
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h758, _) => {
                            h758
                        }
                    }, match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h759) => {
                            h759
                        }
                    })
                } else {
                    Adt7::C0(match v0.clone() {
                        Adt7::C0(h760, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h760
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, h761, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h761
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, h762, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h762
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, h763, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h763
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, h764, _, _, _, _, _, _, _, _, _, _, _) => {
                            h764
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, h765, _, _, _, _, _, _, _, _, _, _) => {
                            h765
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, h766, _, _, _, _, _, _, _, _, _) => {
                            h766
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, h767, _, _, _, _, _, _, _, _) => {
                            h767
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, h768, _, _, _, _, _, _, _) => {
                            h768
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, h769, _, _, _, _, _, _) => {
                            h769
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, h770, _, _, _, _, _) => {
                            h770
                        }
                    }, v2, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, h771, _, _, _) => {
                            h771
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h772, _, _) => {
                            h772
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h773, _) => {
                            h773
                        }
                    }, match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h774) => {
                            h774
                        }
                    })
                }
            }
        } else {
            let a775 = v1;
            let a776 = 14u64;
            if nat_lt(a775, a776) {
                let a777 = v1;
                let a778 = 13u64;
                if nat_lt(a777, a778) {
                    Adt7::C0(match v0.clone() {
                        Adt7::C0(h779, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h779
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, h780, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h780
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, h781, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h781
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, h782, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h782
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, h783, _, _, _, _, _, _, _, _, _, _, _) => {
                            h783
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, h784, _, _, _, _, _, _, _, _, _, _) => {
                            h784
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, h785, _, _, _, _, _, _, _, _, _) => {
                            h785
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, h786, _, _, _, _, _, _, _, _) => {
                            h786
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, h787, _, _, _, _, _, _, _) => {
                            h787
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, h788, _, _, _, _, _, _) => {
                            h788
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, h789, _, _, _, _, _) => {
                            h789
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, h790, _, _, _, _) => {
                            h790
                        }
                    }, v2, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h791, _, _) => {
                            h791
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h792, _) => {
                            h792
                        }
                    }, match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h793) => {
                            h793
                        }
                    })
                } else {
                    Adt7::C0(match v0.clone() {
                        Adt7::C0(h794, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h794
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, h795, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h795
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, h796, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h796
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, h797, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h797
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, h798, _, _, _, _, _, _, _, _, _, _, _) => {
                            h798
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, h799, _, _, _, _, _, _, _, _, _, _) => {
                            h799
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, h800, _, _, _, _, _, _, _, _, _) => {
                            h800
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, h801, _, _, _, _, _, _, _, _) => {
                            h801
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, h802, _, _, _, _, _, _, _) => {
                            h802
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, h803, _, _, _, _, _, _) => {
                            h803
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, h804, _, _, _, _, _) => {
                            h804
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, h805, _, _, _, _) => {
                            h805
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, h806, _, _, _) => {
                            h806
                        }
                    }, v2, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h807, _) => {
                            h807
                        }
                    }, match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h808) => {
                            h808
                        }
                    })
                }
            } else {
                let a809 = v1;
                let a810 = 15u64;
                if nat_lt(a809, a810) {
                    Adt7::C0(match v0.clone() {
                        Adt7::C0(h811, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h811
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, h812, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h812
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, h813, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h813
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, h814, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h814
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, h815, _, _, _, _, _, _, _, _, _, _, _) => {
                            h815
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, h816, _, _, _, _, _, _, _, _, _, _) => {
                            h816
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, h817, _, _, _, _, _, _, _, _, _) => {
                            h817
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, h818, _, _, _, _, _, _, _, _) => {
                            h818
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, h819, _, _, _, _, _, _, _) => {
                            h819
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, h820, _, _, _, _, _, _) => {
                            h820
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, h821, _, _, _, _, _) => {
                            h821
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, h822, _, _, _, _) => {
                            h822
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, h823, _, _, _) => {
                            h823
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h824, _, _) => {
                            h824
                        }
                    }, v2, match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h825) => {
                            h825
                        }
                    })
                } else {
                    Adt7::C0(match v0.clone() {
                        Adt7::C0(h826, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h826
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, h827, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h827
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, h828, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h828
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, h829, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h829
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, h830, _, _, _, _, _, _, _, _, _, _, _) => {
                            h830
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, h831, _, _, _, _, _, _, _, _, _, _) => {
                            h831
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, h832, _, _, _, _, _, _, _, _, _) => {
                            h832
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, h833, _, _, _, _, _, _, _, _) => {
                            h833
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, h834, _, _, _, _, _, _, _) => {
                            h834
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, h835, _, _, _, _, _, _) => {
                            h835
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, h836, _, _, _, _, _) => {
                            h836
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, h837, _, _, _, _) => {
                            h837
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, h838, _, _, _) => {
                            h838
                        }
                    }, match v0.clone() {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h839, _, _) => {
                            h839
                        }
                    }, match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h840, _) => {
                            h840
                        }
                    }, v2)
                }
            }
        }
    }
}

pub fn f70(v0: Adt3, v1: Adt7, v2: Adt15, v3: u64) -> R<Adt15> {
    let v4: u64 = {
        let a841 = v3;
        let a842 = 784u64;
        if nat_lt(a841, a842) {
            let a843 = v3;
            let a844 = 112u64;
            let a845 = 0u64;
            nat_quot(a843, a844, a845)
        } else {
            7u64
        }
    };
    let v5: Adt17 = f73({
        let a846 = v3;
        let a847 = 784u64;
        if nat_lt(a846, a847) {
            let a848 = v3;
            let a849 = 112u64;
            let a850 = 0u64;
            nat_rem(a848, a849, a850)
        } else {
            let a851 = v3;
            let a852 = 672u64;
            nat_sub(a851, a852)
        }
    });
    let v6: Adt7 = match v2.clone() {
        Adt15::C0(h853, _, _) => {
            h853
        }
    };
    let v7: u32 = f74(v6.clone(), match v5.clone() {
        Adt17::C0(_, _, _, h854, _) => {
            h854
        }
    });
    let v8: u32 = {
        let m856 = match v5.clone() {
            Adt17::C0(_, _, h855, _, _) => {
                h855
            }
        };
        match m856 {
            Adt14::C0 => {
                f74(v6.clone(), match v5.clone() {
                    Adt17::C0(_, _, _, _, h857) => {
                        h857
                    }
                })
            }
            Adt14::C1 => {
                f74(v1, f75(v4, match v5.clone() {
                    Adt17::C0(_, _, _, _, h858) => {
                        h858
                    }
                })?)
            }
            Adt14::C2 => {
                f74(v6.clone(), match v5.clone() {
                    Adt17::C0(_, _, _, _, h859) => {
                        h859
                    }
                })
            }
            Adt14::C3 => {
                f76(match v5.clone() {
                    Adt17::C0(_, _, _, _, h860) => {
                        h860
                    }
                })
            }
            Adt14::C4 => {
                f77(v0, match v5.clone() {
                    Adt17::C0(_, _, _, _, h861) => {
                        h861
                    }
                })
            }
        }
    };
    let v9: u32 = f78(match v5.clone() {
        Adt17::C0(_, _, h862, _, _) => {
            h862
        }
    }, v7, v8);
    Ok(Adt15::C0({
        let a863 = v4;
        let a864 = 7u64;
        if nat_eq(a863, a864) {
            v6
        } else {
            f69(v6, match v5.clone() {
                Adt17::C0(_, _, _, h865, _) => {
                    h865
                }
            }, v9)
        }
    }, {
        let a866 = v4;
        let a867 = 7u64;
        if nat_eq(a866, a867) {
            f69(match v2.clone() {
                Adt15::C0(_, h868, _) => {
                    h868
                }
            }, match v5.clone() {
                Adt17::C0(_, _, _, h869, _) => {
                    h869
                }
            }, v9)
        } else {
            match v2.clone() {
                Adt15::C0(_, h870, _) => {
                    h870
                }
            }
        }
    }, List::cons(Adt13::C0(v4, match v5.clone() {
        Adt17::C0(h871, _, _, _, _) => {
            h871
        }
    }, match v5.clone() {
        Adt17::C0(_, h872, _, _, _) => {
            h872
        }
    }, match v5.clone() {
        Adt17::C0(_, _, h873, _, _) => {
            h873
        }
    }, match v5.clone() {
        Adt17::C0(_, _, _, h874, _) => {
            h874
        }
    }, match v5 {
        Adt17::C0(_, _, _, _, h875) => {
            h875
        }
    }, v7, v8, v9), match v2 {
        Adt15::C0(_, _, h876) => {
            h876
        }
    })))
}

pub fn f71(v0: Fn11, p881: u64, p882: Adt16) -> R<(Adt16, bool)> {
    let m880 = (p881, p882);
    {
        let mut m880 = m880;
        loop {
            let step = {
                let (v1, v2) = m880;
                let m877 = v1;
                if m877 == 0 {
                    Err::<(u64, Adt16), R<(Adt16, bool)>>(Ok((v2, false)))
                } else {
                    let v3 = m877 - 1;
                    let m879 = {
                        let c878: Fn11 = v0.clone();
                        c878.apply(v2.clone())?
                    };
                    match m879 {
                        None => {
                            Err::<(u64, Adt16), R<(Adt16, bool)>>(Ok((v2, true)))
                        }
                        Some(v4) => {
                            Ok::<(u64, Adt16), R<(Adt16, bool)>>((v3, v4))
                        }
                    }
                }
            };
            match step {
                Ok(next) => { m880 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f72(v0: u32, v1: Adt3, v2: Adt16) -> R<Option<Adt16>> {
    f79(v1, v0, v2)
}

pub fn f73(v0: u64) -> Adt17 {
    let a883 = v0;
    let a884 = 64u64;
    if nat_lt(a883, a884) {
        let a885 = v0;
        let a886 = 32u64;
        if nat_lt(a885, a886) {
            let a887 = v0;
            let a888 = 16u64;
            if nat_lt(a887, a888) {
                let a889 = v0;
                let a890 = 8u64;
                if nat_lt(a889, a890) {
                    let a891 = v0;
                    let a892 = 4u64;
                    if nat_lt(a891, a892) {
                        let a893 = v0;
                        let a894 = 2u64;
                        if nat_lt(a893, a894) {
                            let a895 = v0;
                            let a896 = 1u64;
                            if nat_lt(a895, a896) {
                                Adt17::C0(0u64, 0u64, Adt14::C0, 0u64, 4u64)
                            } else {
                                Adt17::C0(0u64, 1u64, Adt14::C1, 0u64, 0u64)
                            }
                        } else {
                            let a897 = v0;
                            let a898 = 3u64;
                            if nat_lt(a897, a898) {
                                Adt17::C0(0u64, 2u64, Adt14::C2, 12u64, 0u64)
                            } else {
                                Adt17::C0(0u64, 3u64, Adt14::C3, 12u64, 16u64)
                            }
                        }
                    } else {
                        let a899 = v0;
                        let a900 = 6u64;
                        if nat_lt(a899, a900) {
                            let a901 = v0;
                            let a902 = 5u64;
                            if nat_lt(a901, a902) {
                                Adt17::C0(0u64, 4u64, Adt14::C0, 8u64, 12u64)
                            } else {
                                Adt17::C0(0u64, 5u64, Adt14::C2, 4u64, 8u64)
                            }
                        } else {
                            let a903 = v0;
                            let a904 = 7u64;
                            if nat_lt(a903, a904) {
                                Adt17::C0(0u64, 6u64, Adt14::C3, 4u64, 12u64)
                            } else {
                                Adt17::C0(0u64, 7u64, Adt14::C0, 0u64, 4u64)
                            }
                        }
                    }
                } else {
                    let a905 = v0;
                    let a906 = 12u64;
                    if nat_lt(a905, a906) {
                        let a907 = v0;
                        let a908 = 10u64;
                        if nat_lt(a907, a908) {
                            let a909 = v0;
                            let a910 = 9u64;
                            if nat_lt(a909, a910) {
                                Adt17::C0(0u64, 8u64, Adt14::C1, 0u64, 1u64)
                            } else {
                                Adt17::C0(0u64, 9u64, Adt14::C2, 12u64, 0u64)
                            }
                        } else {
                            let a911 = v0;
                            let a912 = 11u64;
                            if nat_lt(a911, a912) {
                                Adt17::C0(0u64, 10u64, Adt14::C3, 12u64, 8u64)
                            } else {
                                Adt17::C0(0u64, 11u64, Adt14::C0, 8u64, 12u64)
                            }
                        }
                    } else {
                        let a913 = v0;
                        let a914 = 14u64;
                        if nat_lt(a913, a914) {
                            let a915 = v0;
                            let a916 = 13u64;
                            if nat_lt(a915, a916) {
                                Adt17::C0(0u64, 12u64, Adt14::C2, 4u64, 8u64)
                            } else {
                                Adt17::C0(0u64, 13u64, Adt14::C3, 4u64, 7u64)
                            }
                        } else {
                            let a917 = v0;
                            let a918 = 15u64;
                            if nat_lt(a917, a918) {
                                Adt17::C0(1u64, 0u64, Adt14::C0, 1u64, 5u64)
                            } else {
                                Adt17::C0(1u64, 1u64, Adt14::C1, 1u64, 2u64)
                            }
                        }
                    }
                }
            } else {
                let a919 = v0;
                let a920 = 24u64;
                if nat_lt(a919, a920) {
                    let a921 = v0;
                    let a922 = 20u64;
                    if nat_lt(a921, a922) {
                        let a923 = v0;
                        let a924 = 18u64;
                        if nat_lt(a923, a924) {
                            let a925 = v0;
                            let a926 = 17u64;
                            if nat_lt(a925, a926) {
                                Adt17::C0(1u64, 2u64, Adt14::C2, 13u64, 1u64)
                            } else {
                                Adt17::C0(1u64, 3u64, Adt14::C3, 13u64, 16u64)
                            }
                        } else {
                            let a927 = v0;
                            let a928 = 19u64;
                            if nat_lt(a927, a928) {
                                Adt17::C0(1u64, 4u64, Adt14::C0, 9u64, 13u64)
                            } else {
                                Adt17::C0(1u64, 5u64, Adt14::C2, 5u64, 9u64)
                            }
                        }
                    } else {
                        let a929 = v0;
                        let a930 = 22u64;
                        if nat_lt(a929, a930) {
                            let a931 = v0;
                            let a932 = 21u64;
                            if nat_lt(a931, a932) {
                                Adt17::C0(1u64, 6u64, Adt14::C3, 5u64, 12u64)
                            } else {
                                Adt17::C0(1u64, 7u64, Adt14::C0, 1u64, 5u64)
                            }
                        } else {
                            let a933 = v0;
                            let a934 = 23u64;
                            if nat_lt(a933, a934) {
                                Adt17::C0(1u64, 8u64, Adt14::C1, 1u64, 3u64)
                            } else {
                                Adt17::C0(1u64, 9u64, Adt14::C2, 13u64, 1u64)
                            }
                        }
                    }
                } else {
                    let a935 = v0;
                    let a936 = 28u64;
                    if nat_lt(a935, a936) {
                        let a937 = v0;
                        let a938 = 26u64;
                        if nat_lt(a937, a938) {
                            let a939 = v0;
                            let a940 = 25u64;
                            if nat_lt(a939, a940) {
                                Adt17::C0(1u64, 10u64, Adt14::C3, 13u64, 8u64)
                            } else {
                                Adt17::C0(1u64, 11u64, Adt14::C0, 9u64, 13u64)
                            }
                        } else {
                            let a941 = v0;
                            let a942 = 27u64;
                            if nat_lt(a941, a942) {
                                Adt17::C0(1u64, 12u64, Adt14::C2, 5u64, 9u64)
                            } else {
                                Adt17::C0(1u64, 13u64, Adt14::C3, 5u64, 7u64)
                            }
                        }
                    } else {
                        let a943 = v0;
                        let a944 = 30u64;
                        if nat_lt(a943, a944) {
                            let a945 = v0;
                            let a946 = 29u64;
                            if nat_lt(a945, a946) {
                                Adt17::C0(2u64, 0u64, Adt14::C0, 2u64, 6u64)
                            } else {
                                Adt17::C0(2u64, 1u64, Adt14::C1, 2u64, 4u64)
                            }
                        } else {
                            let a947 = v0;
                            let a948 = 31u64;
                            if nat_lt(a947, a948) {
                                Adt17::C0(2u64, 2u64, Adt14::C2, 14u64, 2u64)
                            } else {
                                Adt17::C0(2u64, 3u64, Adt14::C3, 14u64, 16u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a949 = v0;
            let a950 = 48u64;
            if nat_lt(a949, a950) {
                let a951 = v0;
                let a952 = 40u64;
                if nat_lt(a951, a952) {
                    let a953 = v0;
                    let a954 = 36u64;
                    if nat_lt(a953, a954) {
                        let a955 = v0;
                        let a956 = 34u64;
                        if nat_lt(a955, a956) {
                            let a957 = v0;
                            let a958 = 33u64;
                            if nat_lt(a957, a958) {
                                Adt17::C0(2u64, 4u64, Adt14::C0, 10u64, 14u64)
                            } else {
                                Adt17::C0(2u64, 5u64, Adt14::C2, 6u64, 10u64)
                            }
                        } else {
                            let a959 = v0;
                            let a960 = 35u64;
                            if nat_lt(a959, a960) {
                                Adt17::C0(2u64, 6u64, Adt14::C3, 6u64, 12u64)
                            } else {
                                Adt17::C0(2u64, 7u64, Adt14::C0, 2u64, 6u64)
                            }
                        }
                    } else {
                        let a961 = v0;
                        let a962 = 38u64;
                        if nat_lt(a961, a962) {
                            let a963 = v0;
                            let a964 = 37u64;
                            if nat_lt(a963, a964) {
                                Adt17::C0(2u64, 8u64, Adt14::C1, 2u64, 5u64)
                            } else {
                                Adt17::C0(2u64, 9u64, Adt14::C2, 14u64, 2u64)
                            }
                        } else {
                            let a965 = v0;
                            let a966 = 39u64;
                            if nat_lt(a965, a966) {
                                Adt17::C0(2u64, 10u64, Adt14::C3, 14u64, 8u64)
                            } else {
                                Adt17::C0(2u64, 11u64, Adt14::C0, 10u64, 14u64)
                            }
                        }
                    }
                } else {
                    let a967 = v0;
                    let a968 = 44u64;
                    if nat_lt(a967, a968) {
                        let a969 = v0;
                        let a970 = 42u64;
                        if nat_lt(a969, a970) {
                            let a971 = v0;
                            let a972 = 41u64;
                            if nat_lt(a971, a972) {
                                Adt17::C0(2u64, 12u64, Adt14::C2, 6u64, 10u64)
                            } else {
                                Adt17::C0(2u64, 13u64, Adt14::C3, 6u64, 7u64)
                            }
                        } else {
                            let a973 = v0;
                            let a974 = 43u64;
                            if nat_lt(a973, a974) {
                                Adt17::C0(3u64, 0u64, Adt14::C0, 3u64, 7u64)
                            } else {
                                Adt17::C0(3u64, 1u64, Adt14::C1, 3u64, 6u64)
                            }
                        }
                    } else {
                        let a975 = v0;
                        let a976 = 46u64;
                        if nat_lt(a975, a976) {
                            let a977 = v0;
                            let a978 = 45u64;
                            if nat_lt(a977, a978) {
                                Adt17::C0(3u64, 2u64, Adt14::C2, 15u64, 3u64)
                            } else {
                                Adt17::C0(3u64, 3u64, Adt14::C3, 15u64, 16u64)
                            }
                        } else {
                            let a979 = v0;
                            let a980 = 47u64;
                            if nat_lt(a979, a980) {
                                Adt17::C0(3u64, 4u64, Adt14::C0, 11u64, 15u64)
                            } else {
                                Adt17::C0(3u64, 5u64, Adt14::C2, 7u64, 11u64)
                            }
                        }
                    }
                }
            } else {
                let a981 = v0;
                let a982 = 56u64;
                if nat_lt(a981, a982) {
                    let a983 = v0;
                    let a984 = 52u64;
                    if nat_lt(a983, a984) {
                        let a985 = v0;
                        let a986 = 50u64;
                        if nat_lt(a985, a986) {
                            let a987 = v0;
                            let a988 = 49u64;
                            if nat_lt(a987, a988) {
                                Adt17::C0(3u64, 6u64, Adt14::C3, 7u64, 12u64)
                            } else {
                                Adt17::C0(3u64, 7u64, Adt14::C0, 3u64, 7u64)
                            }
                        } else {
                            let a989 = v0;
                            let a990 = 51u64;
                            if nat_lt(a989, a990) {
                                Adt17::C0(3u64, 8u64, Adt14::C1, 3u64, 7u64)
                            } else {
                                Adt17::C0(3u64, 9u64, Adt14::C2, 15u64, 3u64)
                            }
                        }
                    } else {
                        let a991 = v0;
                        let a992 = 54u64;
                        if nat_lt(a991, a992) {
                            let a993 = v0;
                            let a994 = 53u64;
                            if nat_lt(a993, a994) {
                                Adt17::C0(3u64, 10u64, Adt14::C3, 15u64, 8u64)
                            } else {
                                Adt17::C0(3u64, 11u64, Adt14::C0, 11u64, 15u64)
                            }
                        } else {
                            let a995 = v0;
                            let a996 = 55u64;
                            if nat_lt(a995, a996) {
                                Adt17::C0(3u64, 12u64, Adt14::C2, 7u64, 11u64)
                            } else {
                                Adt17::C0(3u64, 13u64, Adt14::C3, 7u64, 7u64)
                            }
                        }
                    }
                } else {
                    let a997 = v0;
                    let a998 = 60u64;
                    if nat_lt(a997, a998) {
                        let a999 = v0;
                        let a1000 = 58u64;
                        if nat_lt(a999, a1000) {
                            let a1001 = v0;
                            let a1002 = 57u64;
                            if nat_lt(a1001, a1002) {
                                Adt17::C0(4u64, 0u64, Adt14::C0, 0u64, 5u64)
                            } else {
                                Adt17::C0(4u64, 1u64, Adt14::C1, 0u64, 8u64)
                            }
                        } else {
                            let a1003 = v0;
                            let a1004 = 59u64;
                            if nat_lt(a1003, a1004) {
                                Adt17::C0(4u64, 2u64, Adt14::C2, 15u64, 0u64)
                            } else {
                                Adt17::C0(4u64, 3u64, Adt14::C3, 15u64, 16u64)
                            }
                        }
                    } else {
                        let a1005 = v0;
                        let a1006 = 62u64;
                        if nat_lt(a1005, a1006) {
                            let a1007 = v0;
                            let a1008 = 61u64;
                            if nat_lt(a1007, a1008) {
                                Adt17::C0(4u64, 4u64, Adt14::C0, 10u64, 15u64)
                            } else {
                                Adt17::C0(4u64, 5u64, Adt14::C2, 5u64, 10u64)
                            }
                        } else {
                            let a1009 = v0;
                            let a1010 = 63u64;
                            if nat_lt(a1009, a1010) {
                                Adt17::C0(4u64, 6u64, Adt14::C3, 5u64, 12u64)
                            } else {
                                Adt17::C0(4u64, 7u64, Adt14::C0, 0u64, 5u64)
                            }
                        }
                    }
                }
            }
        }
    } else {
        let a1011 = v0;
        let a1012 = 96u64;
        if nat_lt(a1011, a1012) {
            let a1013 = v0;
            let a1014 = 80u64;
            if nat_lt(a1013, a1014) {
                let a1015 = v0;
                let a1016 = 72u64;
                if nat_lt(a1015, a1016) {
                    let a1017 = v0;
                    let a1018 = 68u64;
                    if nat_lt(a1017, a1018) {
                        let a1019 = v0;
                        let a1020 = 66u64;
                        if nat_lt(a1019, a1020) {
                            let a1021 = v0;
                            let a1022 = 65u64;
                            if nat_lt(a1021, a1022) {
                                Adt17::C0(4u64, 8u64, Adt14::C1, 0u64, 9u64)
                            } else {
                                Adt17::C0(4u64, 9u64, Adt14::C2, 15u64, 0u64)
                            }
                        } else {
                            let a1023 = v0;
                            let a1024 = 67u64;
                            if nat_lt(a1023, a1024) {
                                Adt17::C0(4u64, 10u64, Adt14::C3, 15u64, 8u64)
                            } else {
                                Adt17::C0(4u64, 11u64, Adt14::C0, 10u64, 15u64)
                            }
                        }
                    } else {
                        let a1025 = v0;
                        let a1026 = 70u64;
                        if nat_lt(a1025, a1026) {
                            let a1027 = v0;
                            let a1028 = 69u64;
                            if nat_lt(a1027, a1028) {
                                Adt17::C0(4u64, 12u64, Adt14::C2, 5u64, 10u64)
                            } else {
                                Adt17::C0(4u64, 13u64, Adt14::C3, 5u64, 7u64)
                            }
                        } else {
                            let a1029 = v0;
                            let a1030 = 71u64;
                            if nat_lt(a1029, a1030) {
                                Adt17::C0(5u64, 0u64, Adt14::C0, 1u64, 6u64)
                            } else {
                                Adt17::C0(5u64, 1u64, Adt14::C1, 1u64, 10u64)
                            }
                        }
                    }
                } else {
                    let a1031 = v0;
                    let a1032 = 76u64;
                    if nat_lt(a1031, a1032) {
                        let a1033 = v0;
                        let a1034 = 74u64;
                        if nat_lt(a1033, a1034) {
                            let a1035 = v0;
                            let a1036 = 73u64;
                            if nat_lt(a1035, a1036) {
                                Adt17::C0(5u64, 2u64, Adt14::C2, 12u64, 1u64)
                            } else {
                                Adt17::C0(5u64, 3u64, Adt14::C3, 12u64, 16u64)
                            }
                        } else {
                            let a1037 = v0;
                            let a1038 = 75u64;
                            if nat_lt(a1037, a1038) {
                                Adt17::C0(5u64, 4u64, Adt14::C0, 11u64, 12u64)
                            } else {
                                Adt17::C0(5u64, 5u64, Adt14::C2, 6u64, 11u64)
                            }
                        }
                    } else {
                        let a1039 = v0;
                        let a1040 = 78u64;
                        if nat_lt(a1039, a1040) {
                            let a1041 = v0;
                            let a1042 = 77u64;
                            if nat_lt(a1041, a1042) {
                                Adt17::C0(5u64, 6u64, Adt14::C3, 6u64, 12u64)
                            } else {
                                Adt17::C0(5u64, 7u64, Adt14::C0, 1u64, 6u64)
                            }
                        } else {
                            let a1043 = v0;
                            let a1044 = 79u64;
                            if nat_lt(a1043, a1044) {
                                Adt17::C0(5u64, 8u64, Adt14::C1, 1u64, 11u64)
                            } else {
                                Adt17::C0(5u64, 9u64, Adt14::C2, 12u64, 1u64)
                            }
                        }
                    }
                }
            } else {
                let a1045 = v0;
                let a1046 = 88u64;
                if nat_lt(a1045, a1046) {
                    let a1047 = v0;
                    let a1048 = 84u64;
                    if nat_lt(a1047, a1048) {
                        let a1049 = v0;
                        let a1050 = 82u64;
                        if nat_lt(a1049, a1050) {
                            let a1051 = v0;
                            let a1052 = 81u64;
                            if nat_lt(a1051, a1052) {
                                Adt17::C0(5u64, 10u64, Adt14::C3, 12u64, 8u64)
                            } else {
                                Adt17::C0(5u64, 11u64, Adt14::C0, 11u64, 12u64)
                            }
                        } else {
                            let a1053 = v0;
                            let a1054 = 83u64;
                            if nat_lt(a1053, a1054) {
                                Adt17::C0(5u64, 12u64, Adt14::C2, 6u64, 11u64)
                            } else {
                                Adt17::C0(5u64, 13u64, Adt14::C3, 6u64, 7u64)
                            }
                        }
                    } else {
                        let a1055 = v0;
                        let a1056 = 86u64;
                        if nat_lt(a1055, a1056) {
                            let a1057 = v0;
                            let a1058 = 85u64;
                            if nat_lt(a1057, a1058) {
                                Adt17::C0(6u64, 0u64, Adt14::C0, 2u64, 7u64)
                            } else {
                                Adt17::C0(6u64, 1u64, Adt14::C1, 2u64, 12u64)
                            }
                        } else {
                            let a1059 = v0;
                            let a1060 = 87u64;
                            if nat_lt(a1059, a1060) {
                                Adt17::C0(6u64, 2u64, Adt14::C2, 13u64, 2u64)
                            } else {
                                Adt17::C0(6u64, 3u64, Adt14::C3, 13u64, 16u64)
                            }
                        }
                    }
                } else {
                    let a1061 = v0;
                    let a1062 = 92u64;
                    if nat_lt(a1061, a1062) {
                        let a1063 = v0;
                        let a1064 = 90u64;
                        if nat_lt(a1063, a1064) {
                            let a1065 = v0;
                            let a1066 = 89u64;
                            if nat_lt(a1065, a1066) {
                                Adt17::C0(6u64, 4u64, Adt14::C0, 8u64, 13u64)
                            } else {
                                Adt17::C0(6u64, 5u64, Adt14::C2, 7u64, 8u64)
                            }
                        } else {
                            let a1067 = v0;
                            let a1068 = 91u64;
                            if nat_lt(a1067, a1068) {
                                Adt17::C0(6u64, 6u64, Adt14::C3, 7u64, 12u64)
                            } else {
                                Adt17::C0(6u64, 7u64, Adt14::C0, 2u64, 7u64)
                            }
                        }
                    } else {
                        let a1069 = v0;
                        let a1070 = 94u64;
                        if nat_lt(a1069, a1070) {
                            let a1071 = v0;
                            let a1072 = 93u64;
                            if nat_lt(a1071, a1072) {
                                Adt17::C0(6u64, 8u64, Adt14::C1, 2u64, 13u64)
                            } else {
                                Adt17::C0(6u64, 9u64, Adt14::C2, 13u64, 2u64)
                            }
                        } else {
                            let a1073 = v0;
                            let a1074 = 95u64;
                            if nat_lt(a1073, a1074) {
                                Adt17::C0(6u64, 10u64, Adt14::C3, 13u64, 8u64)
                            } else {
                                Adt17::C0(6u64, 11u64, Adt14::C0, 8u64, 13u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a1075 = v0;
            let a1076 = 112u64;
            if nat_lt(a1075, a1076) {
                let a1077 = v0;
                let a1078 = 104u64;
                if nat_lt(a1077, a1078) {
                    let a1079 = v0;
                    let a1080 = 100u64;
                    if nat_lt(a1079, a1080) {
                        let a1081 = v0;
                        let a1082 = 98u64;
                        if nat_lt(a1081, a1082) {
                            let a1083 = v0;
                            let a1084 = 97u64;
                            if nat_lt(a1083, a1084) {
                                Adt17::C0(6u64, 12u64, Adt14::C2, 7u64, 8u64)
                            } else {
                                Adt17::C0(6u64, 13u64, Adt14::C3, 7u64, 7u64)
                            }
                        } else {
                            let a1085 = v0;
                            let a1086 = 99u64;
                            if nat_lt(a1085, a1086) {
                                Adt17::C0(7u64, 0u64, Adt14::C0, 3u64, 4u64)
                            } else {
                                Adt17::C0(7u64, 1u64, Adt14::C1, 3u64, 14u64)
                            }
                        }
                    } else {
                        let a1087 = v0;
                        let a1088 = 102u64;
                        if nat_lt(a1087, a1088) {
                            let a1089 = v0;
                            let a1090 = 101u64;
                            if nat_lt(a1089, a1090) {
                                Adt17::C0(7u64, 2u64, Adt14::C2, 14u64, 3u64)
                            } else {
                                Adt17::C0(7u64, 3u64, Adt14::C3, 14u64, 16u64)
                            }
                        } else {
                            let a1091 = v0;
                            let a1092 = 103u64;
                            if nat_lt(a1091, a1092) {
                                Adt17::C0(7u64, 4u64, Adt14::C0, 9u64, 14u64)
                            } else {
                                Adt17::C0(7u64, 5u64, Adt14::C2, 4u64, 9u64)
                            }
                        }
                    }
                } else {
                    let a1093 = v0;
                    let a1094 = 108u64;
                    if nat_lt(a1093, a1094) {
                        let a1095 = v0;
                        let a1096 = 106u64;
                        if nat_lt(a1095, a1096) {
                            let a1097 = v0;
                            let a1098 = 105u64;
                            if nat_lt(a1097, a1098) {
                                Adt17::C0(7u64, 6u64, Adt14::C3, 4u64, 12u64)
                            } else {
                                Adt17::C0(7u64, 7u64, Adt14::C0, 3u64, 4u64)
                            }
                        } else {
                            let a1099 = v0;
                            let a1100 = 107u64;
                            if nat_lt(a1099, a1100) {
                                Adt17::C0(7u64, 8u64, Adt14::C1, 3u64, 15u64)
                            } else {
                                Adt17::C0(7u64, 9u64, Adt14::C2, 14u64, 3u64)
                            }
                        }
                    } else {
                        let a1101 = v0;
                        let a1102 = 110u64;
                        if nat_lt(a1101, a1102) {
                            let a1103 = v0;
                            let a1104 = 109u64;
                            if nat_lt(a1103, a1104) {
                                Adt17::C0(7u64, 10u64, Adt14::C3, 14u64, 8u64)
                            } else {
                                Adt17::C0(7u64, 11u64, Adt14::C0, 9u64, 14u64)
                            }
                        } else {
                            let a1105 = v0;
                            let a1106 = 111u64;
                            if nat_lt(a1105, a1106) {
                                Adt17::C0(7u64, 12u64, Adt14::C2, 4u64, 9u64)
                            } else {
                                Adt17::C0(7u64, 13u64, Adt14::C3, 4u64, 7u64)
                            }
                        }
                    }
                }
            } else {
                let a1107 = v0;
                let a1108 = 120u64;
                if nat_lt(a1107, a1108) {
                    let a1109 = v0;
                    let a1110 = 116u64;
                    if nat_lt(a1109, a1110) {
                        let a1111 = v0;
                        let a1112 = 114u64;
                        if nat_lt(a1111, a1112) {
                            let a1113 = v0;
                            let a1114 = 113u64;
                            if nat_lt(a1113, a1114) {
                                Adt17::C0(0u64, 0u64, Adt14::C2, 0u64, 8u64)
                            } else {
                                Adt17::C0(0u64, 1u64, Adt14::C2, 1u64, 9u64)
                            }
                        } else {
                            let a1115 = v0;
                            let a1116 = 115u64;
                            if nat_lt(a1115, a1116) {
                                Adt17::C0(0u64, 2u64, Adt14::C2, 2u64, 10u64)
                            } else {
                                Adt17::C0(0u64, 3u64, Adt14::C2, 3u64, 11u64)
                            }
                        }
                    } else {
                        let a1117 = v0;
                        let a1118 = 118u64;
                        if nat_lt(a1117, a1118) {
                            let a1119 = v0;
                            let a1120 = 117u64;
                            if nat_lt(a1119, a1120) {
                                Adt17::C0(0u64, 4u64, Adt14::C2, 4u64, 12u64)
                            } else {
                                Adt17::C0(0u64, 5u64, Adt14::C2, 5u64, 13u64)
                            }
                        } else {
                            let a1121 = v0;
                            let a1122 = 119u64;
                            if nat_lt(a1121, a1122) {
                                Adt17::C0(0u64, 6u64, Adt14::C2, 6u64, 14u64)
                            } else {
                                Adt17::C0(0u64, 7u64, Adt14::C2, 7u64, 15u64)
                            }
                        }
                    }
                } else {
                    let a1123 = v0;
                    let a1124 = 124u64;
                    if nat_lt(a1123, a1124) {
                        let a1125 = v0;
                        let a1126 = 122u64;
                        if nat_lt(a1125, a1126) {
                            let a1127 = v0;
                            let a1128 = 121u64;
                            if nat_lt(a1127, a1128) {
                                Adt17::C0(0u64, 8u64, Adt14::C4, 8u64, 0u64)
                            } else {
                                Adt17::C0(0u64, 9u64, Adt14::C4, 9u64, 1u64)
                            }
                        } else {
                            let a1129 = v0;
                            let a1130 = 123u64;
                            if nat_lt(a1129, a1130) {
                                Adt17::C0(0u64, 10u64, Adt14::C4, 10u64, 2u64)
                            } else {
                                Adt17::C0(0u64, 11u64, Adt14::C4, 11u64, 3u64)
                            }
                        }
                    } else {
                        let a1131 = v0;
                        let a1132 = 126u64;
                        if nat_lt(a1131, a1132) {
                            let a1133 = v0;
                            let a1134 = 125u64;
                            if nat_lt(a1133, a1134) {
                                Adt17::C0(0u64, 12u64, Adt14::C4, 12u64, 4u64)
                            } else {
                                Adt17::C0(0u64, 13u64, Adt14::C4, 13u64, 5u64)
                            }
                        } else {
                            let a1135 = v0;
                            let a1136 = 127u64;
                            if nat_lt(a1135, a1136) {
                                Adt17::C0(0u64, 14u64, Adt14::C4, 14u64, 6u64)
                            } else {
                                Adt17::C0(0u64, 15u64, Adt14::C4, 15u64, 7u64)
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn f74(v0: Adt7, v1: u64) -> u32 {
    let a1137 = v1;
    let a1138 = 8u64;
    if nat_lt(a1137, a1138) {
        let a1139 = v1;
        let a1140 = 4u64;
        if nat_lt(a1139, a1140) {
            let a1141 = v1;
            let a1142 = 2u64;
            if nat_lt(a1141, a1142) {
                let a1143 = v1;
                let a1144 = 1u64;
                if nat_lt(a1143, a1144) {
                    match v0 {
                        Adt7::C0(h1145, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1145
                        }
                    }
                } else {
                    match v0 {
                        Adt7::C0(_, h1146, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1146
                        }
                    }
                }
            } else {
                let a1147 = v1;
                let a1148 = 3u64;
                if nat_lt(a1147, a1148) {
                    match v0 {
                        Adt7::C0(_, _, h1149, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1149
                        }
                    }
                } else {
                    match v0 {
                        Adt7::C0(_, _, _, h1150, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1150
                        }
                    }
                }
            }
        } else {
            let a1151 = v1;
            let a1152 = 6u64;
            if nat_lt(a1151, a1152) {
                let a1153 = v1;
                let a1154 = 5u64;
                if nat_lt(a1153, a1154) {
                    match v0 {
                        Adt7::C0(_, _, _, _, h1155, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1155
                        }
                    }
                } else {
                    match v0 {
                        Adt7::C0(_, _, _, _, _, h1156, _, _, _, _, _, _, _, _, _, _) => {
                            h1156
                        }
                    }
                }
            } else {
                let a1157 = v1;
                let a1158 = 7u64;
                if nat_lt(a1157, a1158) {
                    match v0 {
                        Adt7::C0(_, _, _, _, _, _, h1159, _, _, _, _, _, _, _, _, _) => {
                            h1159
                        }
                    }
                } else {
                    match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, h1160, _, _, _, _, _, _, _, _) => {
                            h1160
                        }
                    }
                }
            }
        }
    } else {
        let a1161 = v1;
        let a1162 = 12u64;
        if nat_lt(a1161, a1162) {
            let a1163 = v1;
            let a1164 = 10u64;
            if nat_lt(a1163, a1164) {
                let a1165 = v1;
                let a1166 = 9u64;
                if nat_lt(a1165, a1166) {
                    match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, _, h1167, _, _, _, _, _, _, _) => {
                            h1167
                        }
                    }
                } else {
                    match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, h1168, _, _, _, _, _, _) => {
                            h1168
                        }
                    }
                }
            } else {
                let a1169 = v1;
                let a1170 = 11u64;
                if nat_lt(a1169, a1170) {
                    match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, h1171, _, _, _, _, _) => {
                            h1171
                        }
                    }
                } else {
                    match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, h1172, _, _, _, _) => {
                            h1172
                        }
                    }
                }
            }
        } else {
            let a1173 = v1;
            let a1174 = 14u64;
            if nat_lt(a1173, a1174) {
                let a1175 = v1;
                let a1176 = 13u64;
                if nat_lt(a1175, a1176) {
                    match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1177, _, _, _) => {
                            h1177
                        }
                    }
                } else {
                    match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1178, _, _) => {
                            h1178
                        }
                    }
                }
            } else {
                let a1179 = v1;
                let a1180 = 15u64;
                if nat_lt(a1179, a1180) {
                    match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1181, _) => {
                            h1181
                        }
                    }
                } else {
                    match v0 {
                        Adt7::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1182) => {
                            h1182
                        }
                    }
                }
            }
        }
    }
}

pub fn f75(v0: u64, v1: u64) -> R<u64> {
    let v2: u64 = {
        let a1185 = {
            let a1183 = v0;
            let a1184 = 16u64;
            nat_mul(a1183, a1184)?
        };
        let a1186 = v1;
        nat_add(a1185, a1186)?
    };
    let a1187 = v2;
    let a1188 = 56u64;
    if nat_lt(a1187, a1188) {
        let a1189 = v2;
        let a1190 = 28u64;
        if nat_lt(a1189, a1190) {
            let a1191 = v2;
            let a1192 = 14u64;
            if nat_lt(a1191, a1192) {
                let a1193 = v2;
                let a1194 = 7u64;
                if nat_lt(a1193, a1194) {
                    let a1195 = v2;
                    let a1196 = 3u64;
                    if nat_lt(a1195, a1196) {
                        let a1197 = v2;
                        let a1198 = 1u64;
                        if nat_lt(a1197, a1198) {
                            Ok(0u64)
                        } else {
                            let a1199 = v2;
                            let a1200 = 2u64;
                            if nat_lt(a1199, a1200) {
                                Ok(1u64)
                            } else {
                                Ok(2u64)
                            }
                        }
                    } else {
                        let a1201 = v2;
                        let a1202 = 5u64;
                        if nat_lt(a1201, a1202) {
                            let a1203 = v2;
                            let a1204 = 4u64;
                            if nat_lt(a1203, a1204) {
                                Ok(3u64)
                            } else {
                                Ok(4u64)
                            }
                        } else {
                            let a1205 = v2;
                            let a1206 = 6u64;
                            if nat_lt(a1205, a1206) {
                                Ok(5u64)
                            } else {
                                Ok(6u64)
                            }
                        }
                    }
                } else {
                    let a1207 = v2;
                    let a1208 = 10u64;
                    if nat_lt(a1207, a1208) {
                        let a1209 = v2;
                        let a1210 = 8u64;
                        if nat_lt(a1209, a1210) {
                            Ok(7u64)
                        } else {
                            let a1211 = v2;
                            let a1212 = 9u64;
                            if nat_lt(a1211, a1212) {
                                Ok(8u64)
                            } else {
                                Ok(9u64)
                            }
                        }
                    } else {
                        let a1213 = v2;
                        let a1214 = 12u64;
                        if nat_lt(a1213, a1214) {
                            let a1215 = v2;
                            let a1216 = 11u64;
                            if nat_lt(a1215, a1216) {
                                Ok(10u64)
                            } else {
                                Ok(11u64)
                            }
                        } else {
                            let a1217 = v2;
                            let a1218 = 13u64;
                            if nat_lt(a1217, a1218) {
                                Ok(12u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    }
                }
            } else {
                let a1219 = v2;
                let a1220 = 21u64;
                if nat_lt(a1219, a1220) {
                    let a1221 = v2;
                    let a1222 = 17u64;
                    if nat_lt(a1221, a1222) {
                        let a1223 = v2;
                        let a1224 = 15u64;
                        if nat_lt(a1223, a1224) {
                            Ok(14u64)
                        } else {
                            let a1225 = v2;
                            let a1226 = 16u64;
                            if nat_lt(a1225, a1226) {
                                Ok(15u64)
                            } else {
                                Ok(2u64)
                            }
                        }
                    } else {
                        let a1227 = v2;
                        let a1228 = 19u64;
                        if nat_lt(a1227, a1228) {
                            let a1229 = v2;
                            let a1230 = 18u64;
                            if nat_lt(a1229, a1230) {
                                Ok(6u64)
                            } else {
                                Ok(3u64)
                            }
                        } else {
                            let a1231 = v2;
                            let a1232 = 20u64;
                            if nat_lt(a1231, a1232) {
                                Ok(10u64)
                            } else {
                                Ok(7u64)
                            }
                        }
                    }
                } else {
                    let a1233 = v2;
                    let a1234 = 24u64;
                    if nat_lt(a1233, a1234) {
                        let a1235 = v2;
                        let a1236 = 22u64;
                        if nat_lt(a1235, a1236) {
                            Ok(0u64)
                        } else {
                            let a1237 = v2;
                            let a1238 = 23u64;
                            if nat_lt(a1237, a1238) {
                                Ok(4u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    } else {
                        let a1239 = v2;
                        let a1240 = 26u64;
                        if nat_lt(a1239, a1240) {
                            let a1241 = v2;
                            let a1242 = 25u64;
                            if nat_lt(a1241, a1242) {
                                Ok(1u64)
                            } else {
                                Ok(11u64)
                            }
                        } else {
                            let a1243 = v2;
                            let a1244 = 27u64;
                            if nat_lt(a1243, a1244) {
                                Ok(12u64)
                            } else {
                                Ok(5u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a1245 = v2;
            let a1246 = 42u64;
            if nat_lt(a1245, a1246) {
                let a1247 = v2;
                let a1248 = 35u64;
                if nat_lt(a1247, a1248) {
                    let a1249 = v2;
                    let a1250 = 31u64;
                    if nat_lt(a1249, a1250) {
                        let a1251 = v2;
                        let a1252 = 29u64;
                        if nat_lt(a1251, a1252) {
                            Ok(9u64)
                        } else {
                            let a1253 = v2;
                            let a1254 = 30u64;
                            if nat_lt(a1253, a1254) {
                                Ok(14u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    } else {
                        let a1255 = v2;
                        let a1256 = 33u64;
                        if nat_lt(a1255, a1256) {
                            let a1257 = v2;
                            let a1258 = 32u64;
                            if nat_lt(a1257, a1258) {
                                Ok(8u64)
                            } else {
                                Ok(3u64)
                            }
                        } else {
                            let a1259 = v2;
                            let a1260 = 34u64;
                            if nat_lt(a1259, a1260) {
                                Ok(4u64)
                            } else {
                                Ok(10u64)
                            }
                        }
                    }
                } else {
                    let a1261 = v2;
                    let a1262 = 38u64;
                    if nat_lt(a1261, a1262) {
                        let a1263 = v2;
                        let a1264 = 36u64;
                        if nat_lt(a1263, a1264) {
                            Ok(12u64)
                        } else {
                            let a1265 = v2;
                            let a1266 = 37u64;
                            if nat_lt(a1265, a1266) {
                                Ok(13u64)
                            } else {
                                Ok(2u64)
                            }
                        }
                    } else {
                        let a1267 = v2;
                        let a1268 = 40u64;
                        if nat_lt(a1267, a1268) {
                            let a1269 = v2;
                            let a1270 = 39u64;
                            if nat_lt(a1269, a1270) {
                                Ok(7u64)
                            } else {
                                Ok(14u64)
                            }
                        } else {
                            let a1271 = v2;
                            let a1272 = 41u64;
                            if nat_lt(a1271, a1272) {
                                Ok(6u64)
                            } else {
                                Ok(5u64)
                            }
                        }
                    }
                }
            } else {
                let a1273 = v2;
                let a1274 = 49u64;
                if nat_lt(a1273, a1274) {
                    let a1275 = v2;
                    let a1276 = 45u64;
                    if nat_lt(a1275, a1276) {
                        let a1277 = v2;
                        let a1278 = 43u64;
                        if nat_lt(a1277, a1278) {
                            Ok(9u64)
                        } else {
                            let a1279 = v2;
                            let a1280 = 44u64;
                            if nat_lt(a1279, a1280) {
                                Ok(0u64)
                            } else {
                                Ok(11u64)
                            }
                        }
                    } else {
                        let a1281 = v2;
                        let a1282 = 47u64;
                        if nat_lt(a1281, a1282) {
                            let a1283 = v2;
                            let a1284 = 46u64;
                            if nat_lt(a1283, a1284) {
                                Ok(15u64)
                            } else {
                                Ok(8u64)
                            }
                        } else {
                            let a1285 = v2;
                            let a1286 = 48u64;
                            if nat_lt(a1285, a1286) {
                                Ok(1u64)
                            } else {
                                Ok(10u64)
                            }
                        }
                    }
                } else {
                    let a1287 = v2;
                    let a1288 = 52u64;
                    if nat_lt(a1287, a1288) {
                        let a1289 = v2;
                        let a1290 = 50u64;
                        if nat_lt(a1289, a1290) {
                            Ok(7u64)
                        } else {
                            let a1291 = v2;
                            let a1292 = 51u64;
                            if nat_lt(a1291, a1292) {
                                Ok(12u64)
                            } else {
                                Ok(9u64)
                            }
                        }
                    } else {
                        let a1293 = v2;
                        let a1294 = 54u64;
                        if nat_lt(a1293, a1294) {
                            let a1295 = v2;
                            let a1296 = 53u64;
                            if nat_lt(a1295, a1296) {
                                Ok(14u64)
                            } else {
                                Ok(3u64)
                            }
                        } else {
                            let a1297 = v2;
                            let a1298 = 55u64;
                            if nat_lt(a1297, a1298) {
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
        let a1299 = v2;
        let a1300 = 84u64;
        if nat_lt(a1299, a1300) {
            let a1301 = v2;
            let a1302 = 70u64;
            if nat_lt(a1301, a1302) {
                let a1303 = v2;
                let a1304 = 63u64;
                if nat_lt(a1303, a1304) {
                    let a1305 = v2;
                    let a1306 = 59u64;
                    if nat_lt(a1305, a1306) {
                        let a1307 = v2;
                        let a1308 = 57u64;
                        if nat_lt(a1307, a1308) {
                            Ok(4u64)
                        } else {
                            let a1309 = v2;
                            let a1310 = 58u64;
                            if nat_lt(a1309, a1310) {
                                Ok(0u64)
                            } else {
                                Ok(11u64)
                            }
                        }
                    } else {
                        let a1311 = v2;
                        let a1312 = 61u64;
                        if nat_lt(a1311, a1312) {
                            let a1313 = v2;
                            let a1314 = 60u64;
                            if nat_lt(a1313, a1314) {
                                Ok(2u64)
                            } else {
                                Ok(5u64)
                            }
                        } else {
                            let a1315 = v2;
                            let a1316 = 62u64;
                            if nat_lt(a1315, a1316) {
                                Ok(8u64)
                            } else {
                                Ok(1u64)
                            }
                        }
                    }
                } else {
                    let a1317 = v2;
                    let a1318 = 66u64;
                    if nat_lt(a1317, a1318) {
                        let a1319 = v2;
                        let a1320 = 64u64;
                        if nat_lt(a1319, a1320) {
                            Ok(6u64)
                        } else {
                            let a1321 = v2;
                            let a1322 = 65u64;
                            if nat_lt(a1321, a1322) {
                                Ok(12u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    } else {
                        let a1323 = v2;
                        let a1324 = 68u64;
                        if nat_lt(a1323, a1324) {
                            let a1325 = v2;
                            let a1326 = 67u64;
                            if nat_lt(a1325, a1326) {
                                Ok(9u64)
                            } else {
                                Ok(11u64)
                            }
                        } else {
                            let a1327 = v2;
                            let a1328 = 69u64;
                            if nat_lt(a1327, a1328) {
                                Ok(15u64)
                            } else {
                                Ok(10u64)
                            }
                        }
                    }
                }
            } else {
                let a1329 = v2;
                let a1330 = 77u64;
                if nat_lt(a1329, a1330) {
                    let a1331 = v2;
                    let a1332 = 73u64;
                    if nat_lt(a1331, a1332) {
                        let a1333 = v2;
                        let a1334 = 71u64;
                        if nat_lt(a1333, a1334) {
                            Ok(14u64)
                        } else {
                            let a1335 = v2;
                            let a1336 = 72u64;
                            if nat_lt(a1335, a1336) {
                                Ok(8u64)
                            } else {
                                Ok(7u64)
                            }
                        }
                    } else {
                        let a1337 = v2;
                        let a1338 = 75u64;
                        if nat_lt(a1337, a1338) {
                            let a1339 = v2;
                            let a1340 = 74u64;
                            if nat_lt(a1339, a1340) {
                                Ok(2u64)
                            } else {
                                Ok(5u64)
                            }
                        } else {
                            let a1341 = v2;
                            let a1342 = 76u64;
                            if nat_lt(a1341, a1342) {
                                Ok(3u64)
                            } else {
                                Ok(0u64)
                            }
                        }
                    }
                } else {
                    let a1343 = v2;
                    let a1344 = 80u64;
                    if nat_lt(a1343, a1344) {
                        let a1345 = v2;
                        let a1346 = 78u64;
                        if nat_lt(a1345, a1346) {
                            Ok(1u64)
                        } else {
                            let a1347 = v2;
                            let a1348 = 79u64;
                            if nat_lt(a1347, a1348) {
                                Ok(6u64)
                            } else {
                                Ok(4u64)
                            }
                        }
                    } else {
                        let a1349 = v2;
                        let a1350 = 82u64;
                        if nat_lt(a1349, a1350) {
                            let a1351 = v2;
                            let a1352 = 81u64;
                            if nat_lt(a1351, a1352) {
                                Ok(9u64)
                            } else {
                                Ok(14u64)
                            }
                        } else {
                            let a1353 = v2;
                            let a1354 = 83u64;
                            if nat_lt(a1353, a1354) {
                                Ok(11u64)
                            } else {
                                Ok(5u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a1355 = v2;
            let a1356 = 98u64;
            if nat_lt(a1355, a1356) {
                let a1357 = v2;
                let a1358 = 91u64;
                if nat_lt(a1357, a1358) {
                    let a1359 = v2;
                    let a1360 = 87u64;
                    if nat_lt(a1359, a1360) {
                        let a1361 = v2;
                        let a1362 = 85u64;
                        if nat_lt(a1361, a1362) {
                            Ok(8u64)
                        } else {
                            let a1363 = v2;
                            let a1364 = 86u64;
                            if nat_lt(a1363, a1364) {
                                Ok(12u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    } else {
                        let a1365 = v2;
                        let a1366 = 89u64;
                        if nat_lt(a1365, a1366) {
                            let a1367 = v2;
                            let a1368 = 88u64;
                            if nat_lt(a1367, a1368) {
                                Ok(1u64)
                            } else {
                                Ok(13u64)
                            }
                        } else {
                            let a1369 = v2;
                            let a1370 = 90u64;
                            if nat_lt(a1369, a1370) {
                                Ok(3u64)
                            } else {
                                Ok(0u64)
                            }
                        }
                    }
                } else {
                    let a1371 = v2;
                    let a1372 = 94u64;
                    if nat_lt(a1371, a1372) {
                        let a1373 = v2;
                        let a1374 = 92u64;
                        if nat_lt(a1373, a1374) {
                            Ok(10u64)
                        } else {
                            let a1375 = v2;
                            let a1376 = 93u64;
                            if nat_lt(a1375, a1376) {
                                Ok(2u64)
                            } else {
                                Ok(6u64)
                            }
                        }
                    } else {
                        let a1377 = v2;
                        let a1378 = 96u64;
                        if nat_lt(a1377, a1378) {
                            let a1379 = v2;
                            let a1380 = 95u64;
                            if nat_lt(a1379, a1380) {
                                Ok(4u64)
                            } else {
                                Ok(7u64)
                            }
                        } else {
                            let a1381 = v2;
                            let a1382 = 97u64;
                            if nat_lt(a1381, a1382) {
                                Ok(11u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    }
                }
            } else {
                let a1383 = v2;
                let a1384 = 105u64;
                if nat_lt(a1383, a1384) {
                    let a1385 = v2;
                    let a1386 = 101u64;
                    if nat_lt(a1385, a1386) {
                        let a1387 = v2;
                        let a1388 = 99u64;
                        if nat_lt(a1387, a1388) {
                            Ok(5u64)
                        } else {
                            let a1389 = v2;
                            let a1390 = 100u64;
                            if nat_lt(a1389, a1390) {
                                Ok(0u64)
                            } else {
                                Ok(1u64)
                            }
                        }
                    } else {
                        let a1391 = v2;
                        let a1392 = 103u64;
                        if nat_lt(a1391, a1392) {
                            let a1393 = v2;
                            let a1394 = 102u64;
                            if nat_lt(a1393, a1394) {
                                Ok(9u64)
                            } else {
                                Ok(8u64)
                            }
                        } else {
                            let a1395 = v2;
                            let a1396 = 104u64;
                            if nat_lt(a1395, a1396) {
                                Ok(6u64)
                            } else {
                                Ok(14u64)
                            }
                        }
                    }
                } else {
                    let a1397 = v2;
                    let a1398 = 108u64;
                    if nat_lt(a1397, a1398) {
                        let a1399 = v2;
                        let a1400 = 106u64;
                        if nat_lt(a1399, a1400) {
                            Ok(10u64)
                        } else {
                            let a1401 = v2;
                            let a1402 = 107u64;
                            if nat_lt(a1401, a1402) {
                                Ok(2u64)
                            } else {
                                Ok(12u64)
                            }
                        }
                    } else {
                        let a1403 = v2;
                        let a1404 = 110u64;
                        if nat_lt(a1403, a1404) {
                            let a1405 = v2;
                            let a1406 = 109u64;
                            if nat_lt(a1405, a1406) {
                                Ok(3u64)
                            } else {
                                Ok(4u64)
                            }
                        } else {
                            let a1407 = v2;
                            let a1408 = 111u64;
                            if nat_lt(a1407, a1408) {
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

pub fn f76(v0: u64) -> u32 {
    let a1409 = v0;
    let a1410 = 12u64;
    if nat_lt(a1409, a1410) {
        let a1411 = v0;
        let a1412 = 9u64;
        if nat_lt(a1411, a1412) {
            let a1413 = v0;
            let a1414 = 8u64;
            if nat_lt(a1413, a1414) {
                7u32
            } else {
                8u32
            }
        } else {
            let a1415 = v0;
            let a1416 = 10u64;
            if nat_lt(a1415, a1416) {
                9u32
            } else {
                let a1417 = v0;
                let a1418 = 11u64;
                if nat_lt(a1417, a1418) {
                    10u32
                } else {
                    11u32
                }
            }
        }
    } else {
        let a1419 = v0;
        let a1420 = 14u64;
        if nat_lt(a1419, a1420) {
            let a1421 = v0;
            let a1422 = 13u64;
            if nat_lt(a1421, a1422) {
                12u32
            } else {
                13u32
            }
        } else {
            let a1423 = v0;
            let a1424 = 15u64;
            if nat_lt(a1423, a1424) {
                14u32
            } else {
                let a1425 = v0;
                let a1426 = 16u64;
                if nat_lt(a1425, a1426) {
                    15u32
                } else {
                    16u32
                }
            }
        }
    }
}

pub fn f77(v0: Adt3, v1: u64) -> u32 {
    let a1427 = v1;
    let a1428 = 4u64;
    if nat_lt(a1427, a1428) {
        let a1429 = v1;
        let a1430 = 2u64;
        if nat_lt(a1429, a1430) {
            let a1431 = v1;
            let a1432 = 1u64;
            if nat_lt(a1431, a1432) {
                match v0 {
                    Adt3::C0(h1433, _, _, _, _, _, _, _) => {
                        h1433
                    }
                }
            } else {
                match v0 {
                    Adt3::C0(_, h1434, _, _, _, _, _, _) => {
                        h1434
                    }
                }
            }
        } else {
            let a1435 = v1;
            let a1436 = 3u64;
            if nat_lt(a1435, a1436) {
                match v0 {
                    Adt3::C0(_, _, h1437, _, _, _, _, _) => {
                        h1437
                    }
                }
            } else {
                match v0 {
                    Adt3::C0(_, _, _, h1438, _, _, _, _) => {
                        h1438
                    }
                }
            }
        }
    } else {
        let a1439 = v1;
        let a1440 = 6u64;
        if nat_lt(a1439, a1440) {
            let a1441 = v1;
            let a1442 = 5u64;
            if nat_lt(a1441, a1442) {
                match v0 {
                    Adt3::C0(_, _, _, _, h1443, _, _, _) => {
                        h1443
                    }
                }
            } else {
                match v0 {
                    Adt3::C0(_, _, _, _, _, h1444, _, _) => {
                        h1444
                    }
                }
            }
        } else {
            let a1445 = v1;
            let a1446 = 7u64;
            if nat_lt(a1445, a1446) {
                match v0 {
                    Adt3::C0(_, _, _, _, _, _, h1447, _) => {
                        h1447
                    }
                }
            } else {
                match v0 {
                    Adt3::C0(_, _, _, _, _, _, _, h1448) => {
                        h1448
                    }
                }
            }
        }
    }
}

pub fn f78(v0: Adt14, v1: u32, v2: u32) -> u32 {
    let m1449 = v0;
    match m1449 {
        Adt14::C0 => {
            f80(v1, v2)
        }
        Adt14::C1 => {
            f80(v1, v2)
        }
        Adt14::C2 => {
            f81(v1, v2)
        }
        Adt14::C3 => {
            f82(v1, v2)
        }
        Adt14::C4 => {
            f81(v1, v2)
        }
    }
}

pub fn f79(v0: Adt3, v1: u32, v2: Adt16) -> R<Option<Adt16>> {
    let a1454 = {
        let a1451 = match v2.clone() {
            Adt16::C0(_, h1450, _, _) => {
                h1450
            }
        };
        let a1452 = 2u64;
        let a1453 = 0u64;
        nat_rem(a1451, a1452, a1453)
    };
    let a1455 = 0u64;
    if nat_eq(a1454, a1455) {
        let m1457 = match v2.clone() {
            Adt16::C0(_, _, h1456, _) => {
                h1456
            }
        };
        match m1457.uncons() {
            None => {
                Ok(None::<Adt16>)
            }
            Some((v3, v4)) => {
                let v5: Adt6 = f16(f65(v3.clone(), match v2.clone() {
                    Adt16::C0(h1458, _, _, _) => {
                        h1458
                    }
                }, v0, v1)?)?;
                Ok(Some(Adt16::C0(Adt4::C0(f19(match v5.clone() {
                    Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h1459) => {
                        h1459
                    }
                }), match v3.clone() {
                    Adt4::C0(_, h1460, _) => {
                        h1460
                    }
                }, {
                    let a1465 = match v3.clone() {
                        Adt4::C0(_, _, h1461) => {
                            h1461
                        }
                    };
                    let a1466 = {
                        let m1464 = match v2.clone() {
                            Adt16::C0(h1463, _, _, _) => {
                                h1463
                            }
                        };
                        match m1464 {
                            Adt4::C0(_, _, h1462) => {
                                h1462
                            }
                        }
                    };
                    nat_add(a1465, a1466)?
                }), {
                    let a1468 = match v2.clone() {
                        Adt16::C0(_, h1467, _, _) => {
                            h1467
                        }
                    };
                    let a1469 = 2u64;
                    let a1470 = 0u64;
                    nat_quot(a1468, a1469, a1470)
                }, v4, List::cons(Adt5::C0(v5), List::cons(Adt5::C2(v3, {
                    let a1472 = match v2.clone() {
                        Adt16::C0(_, _, h1471, _) => {
                            h1471
                        }
                    };
                    length_list(a1472)
                }), match v2 {
                    Adt16::C0(_, _, _, h1473) => {
                        h1473
                    }
                })))))
            }
        }
    } else {
        Ok(None::<Adt16>)
    }
}

pub fn f80(v0: u32, v1: u32) -> u32 {
    let m1476 = {
        let a1474 = f83(v0);
        let a1475 = f83(v1);
        fixed_u64::checked_add(a1474, a1475)
    };
    match m1476 {
        None => {
            0u32
        }
        Some(v2) => {
            f84(v2)
        }
    }
}

pub fn f81(v0: u32, v1: u32) -> u32 {
    let a1477 = v0;
    let a1478 = v1;
    fixed_u32::bit_xor(a1477, a1478)
}

pub fn f82(v0: u32, v1: u32) -> u32 {
    let m1481 = {
        let a1479 = 32u32;
        let a1480 = v1;
        fixed_u32::checked_sub(a1479, a1480)
    };
    match m1481 {
        None => {
            v0
        }
        Some(v2) => {
            f85(v0, v1, v2)
        }
    }
}

pub fn f83(v0: u32) -> u64 {
    let m1483 = {
        let a1482 = v0;
        fixed_u64::convert(i128::from(a1482))
    };
    match m1483 {
        None => {
            0u64
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f84(v0: u64) -> u32 {
    let m1487 = {
        let a1486 = {
            let a1484 = v0;
            let a1485 = 4294967295u64;
            fixed_u64::bit_and(a1484, a1485)
        };
        fixed_u32::convert(i128::from(a1486))
    };
    match m1487 {
        None => {
            0u32
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f85(v0: u32, v1: u32, v2: u32) -> u32 {
    let a1488 = f86(v0, v1);
    let a1489 = f41(v0, v2);
    fixed_u32::bit_or(a1488, a1489)
}

pub fn f86(v0: u32, v1: u32) -> u32 {
    let m1492 = {
        let a1490 = v0;
        let a1491 = v1;
        fixed_u32::shift_right(a1490, a1491)
    };
    match m1492 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn run(p0: Adt0, p1: Bytes, p2: u64, p3: u64, p4: Bytes) -> R<Result<Bytes, Adt1>> {
    f0(p0, p1, p2, p3, p4)
}
