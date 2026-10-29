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
}

#[derive(Clone)]
pub enum Adt2 {
    C0(u64, u64, u64, u64, u64, Adt3, Adt4, u32, u32, u32, u32, Adt4),
}

#[derive(Clone)]
pub enum Adt3 {
    C0(u32, u32, u32, u32, u32, u32, u32, u32),
}

#[derive(Clone)]
pub enum Adt4 {
    C0(u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32),
}

#[derive(Clone)]
pub enum Adt5 {
    C0(u64, u64, u64, Adt3, u32, List<Adt6>, List<Adt6>),
}

#[derive(Clone)]
pub enum Adt6 {
    C0(Adt2),
    C1(Adt7, u64),
    C2(Adt7, u64),
}

#[derive(Clone)]
pub enum Adt7 {
    C0(Adt3, u64, u64),
}

#[derive(Clone)]
pub enum Adt8 {
    C0(Adt3, u32, u64, Bytes, List<Adt7>, List<Adt6>, List<Adt6>),
}

#[derive(Clone)]
pub enum Adt9 {
    C0(u64, u64, u64, u64, u64, Adt3, Adt4, u64, u32, u32),
}

#[derive(Clone)]
pub enum Adt10 {
    C0(Adt7, u64, List<Adt7>, List<Adt6>),
}

#[derive(Clone)]
pub enum Adt11 {
    C0(Adt3, Adt4, u32, u32, u32, u32, List<Adt12>),
}

#[derive(Clone)]
pub enum Adt12 {
    C0(u64, u64, u64, Adt13, u64, u64, u32, u32, u32),
}

#[derive(Clone)]
pub enum Adt13 {
    C0,
    C1,
    C2,
    C3,
    C4,
}

#[derive(Clone)]
pub enum Adt14 {
    C0(Adt4, Adt4, List<Adt12>),
}

#[derive(Clone)]
pub enum Adt15 {
    C0(u64, u64, Adt13, u64, u64),
}

#[derive(Clone)]
pub enum Fn0 {
    F5(u64, List<Adt2>),
}

impl Fn0 {
    pub fn apply(&self, p0: (u64, (u64, List<Bytes>))) -> R<Option<(u64, (u64, List<Bytes>))>> {
        match self {
            Fn0::F5(k0, k1) => f5(*k0, k1.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn1 {
    F10,
    F68,
}

impl Fn1 {
    pub fn apply(&self, p0: List<Bytes>, p1: Bytes) -> List<Bytes> {
        match self {
            Fn1::F10 => f10(p0, p1),
            Fn1::F68 => f68(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn2 {
    F14,
}

impl Fn2 {
    pub fn apply(&self, p0: Adt8, p1: Bytes) -> R<Adt8> {
        match self {
            Fn2::F14 => f14(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn3 {
    F16,
}

impl Fn3 {
    pub fn apply(&self, p0: List<Adt2>, p1: Adt6) -> List<Adt2> {
        match self {
            Fn3::F16 => f16(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn4 {
    F19,
}

impl Fn4 {
    pub fn apply(&self, p0: List<Bytes>) -> Option<List<Bytes>> {
        match self {
            Fn4::F19 => f19(p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn5 {
    F30(u32, Adt3),
}

impl Fn5 {
    pub fn apply(&self, p0: (Adt9, (List<Adt6>, u64)), p1: Adt7) -> R<(Adt9, (List<Adt6>, u64))> {
        match self {
            Fn5::F30(k0, k1) => f30(*k0, k1.clone(), p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn6 {
    F32((Adt9, (List<Adt6>, u64))),
}

impl Fn6 {
    pub fn apply(&self, p0: (u64, List<Adt6>)) -> R<(u64, List<Adt6>)> {
        match self {
            Fn6::F32(k0) => f32(k0.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn7 {
    F50(u32, u64, u64, Adt3, u64, Bytes),
}

impl Fn7 {
    pub fn apply(&self, p0: (u64, (Adt3, List<Adt6>))) -> R<(u64, (Adt3, List<Adt6>))> {
        match self {
            Fn7::F50(k0, k1, k2, k3, k4, k5) => f50(*k0, *k1, *k2, k3.clone(), *k4, k5.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn8 {
    F59,
}

impl Fn8 {
    pub fn apply(&self, p0: List<Adt6>, p1: Adt6) -> List<Adt6> {
        match self {
            Fn8::F59 => f59(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn9 {
    F61(u32, Bytes, Adt3, Adt8),
}

impl Fn9 {
    pub fn apply(&self, p0: (u64, (List<Adt7>, List<Adt6>))) -> R<(u64, (List<Adt7>, List<Adt6>))> {
        match self {
            Fn9::F61(k0, k1, k2, k3) => f61(*k0, k1.clone(), k2.clone(), k3.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn10 {
    F63(u64),
}

impl Fn10 {
    pub fn apply(&self, p0: u64) -> R<Option<u64>> {
        match self {
            Fn10::F63(k0) => f63(*k0, p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn11 {
    F65(u64, u64),
}

impl Fn11 {
    pub fn apply(&self, p0: Adt4, p1: Adt2) -> Adt4 {
        match self {
            Fn11::F65(k0, k1) => f65(*k0, *k1, p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn12 {
    F67,
}

impl Fn12 {
    pub fn apply(&self, p0: (Option<Bytes>, List<Bytes>), p1: Bytes) -> (Option<Bytes>, List<Bytes>) {
        match self {
            Fn12::F67 => f67(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn13 {
    F70,
}

impl Fn13 {
    pub fn apply(&self, p0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
        match self {
            Fn13::F70 => f70(p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn14 {
    F73(u64),
}

impl Fn14 {
    pub fn apply(&self, p0: Adt4, p1: Adt6) -> Adt4 {
        match self {
            Fn14::F73(k0) => f73(*k0, p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn15 {
    F86(u32, Adt3),
}

impl Fn15 {
    pub fn apply(&self, p0: Adt10) -> R<Option<Adt10>> {
        match self {
            Fn15::F86(k0, k1) => f86(*k0, k1.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn16 {
    F88,
}

impl Fn16 {
    pub fn apply(&self, p0: Adt4, p1: Adt12) -> Adt4 {
        match self {
            Fn16::F88 => f88(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn17 {
    F90(Adt3, Adt4),
}

impl Fn17 {
    pub fn apply(&self, p0: (u64, Adt14)) -> R<(u64, Adt14)> {
        match self {
            Fn17::F90(k0, k1) => f90(k0.clone(), k1.clone(), p0),
        }
    }
}

pub fn f0(v0: Adt0, v1: Bytes, v2: u64) -> R<Result<Bytes, Adt1>> {
    let m1 = f1(v0, List::cons(v1.clone(), List::<Bytes>::nil()), 32u64)?;
    match m1 {
        Ok(v3) => {
            Ok(Ok::<Bytes, Adt1>({
                let v4: u64 = {
                    let a2 = v1.clone();
                    length_bytes(a2)
                };
                let v5: u64 = f2(v4)?;
                let v6: List<Adt2> = f3(v3);
                let a3 = v2;
                let a4 = v5;
                if nat_lt(a3, a4) {
                    let v7: List<Bytes> = {
                        let (_, h5) = {
                            let (_, h6) = {
                                let (h7, _) = f4(Fn0::F5(v2, v6), 64u64, (0u64, (v5, List::<Bytes>::nil())))?;
                                h7
                            };
                            h6
                        };
                        h5
                    };
                    let v9: Bytes = {
                        let m13 = {
                            let a10 = v1;
                            let a11 = {
                                let a8 = v2;
                                let a9 = 1024u64;
                                nat_mul(a8, a9)?
                            };
                            let a12 = f6(v4, v2)?;
                            slice_bytes(a10, a11, a12)
                        };
                        match m13 {
                            None => {
                                Bytes::lit(&[])
                            }
                            Some(v8) => {
                                v8
                            }
                        }
                    };
                    f7(List::cons(f8(v4), f9(Fn1::F10, List::cons(v9, List::<Bytes>::nil()), v7)))
                } else {
                    Bytes::lit(&[])
                }
            }))
        }
        Err(v10) => {
            Ok(Err::<Bytes, Adt1>(v10))
        }
    }
}

pub fn f1(v0: Adt0, v1: List<Bytes>, v2: u64) -> R<Result<Adt5, Adt1>> {
    let m14 = f11(v0)?;
    match m14 {
        Ok(v3) => {
            Ok(Ok::<Adt5, Adt1>(f12(f13(Fn2::F14, v3, v1)?, v2)?))
        }
        Err(v4) => {
            Ok(Err::<Adt5, Adt1>(v4))
        }
    }
}

pub fn f2(v0: u64) -> R<u64> {
    let a15 = v0;
    let a16 = 0u64;
    if nat_eq(a15, a16) {
        Ok(1u64)
    } else {
        let a22 = {
            let a19 = {
                let a17 = v0;
                let a18 = 1u64;
                nat_sub(a17, a18)
            };
            let a20 = 1024u64;
            let a21 = 0u64;
            nat_quot(a19, a20, a21)
        };
        let a23 = 1u64;
        nat_add(a22, a23)
    }
}

pub fn f3(v0: Adt5) -> List<Adt2> {
    f15(Fn3::F16, List::<Adt2>::nil(), match v0 {
        Adt5::C0(_, _, _, _, _, _, h24) => {
            h24
        }
    })
}

pub fn f4(v0: Fn0, p29: u64, p30: (u64, (u64, List<Bytes>))) -> R<((u64, (u64, List<Bytes>)), bool)> {
    let m28 = (p29, p30);
    {
        let mut m28 = m28;
        loop {
            let step = {
                let (v1, v2) = m28;
                let m25 = v1;
                if m25 == 0 {
                    Err::<(u64, (u64, (u64, List<Bytes>))), R<((u64, (u64, List<Bytes>)), bool)>>(Ok((v2, false)))
                } else {
                    let v3 = m25 - 1;
                    let m27 = {
                        let c26: Fn0 = v0.clone();
                        c26.apply(v2.clone())?
                    };
                    match m27 {
                        None => {
                            Err::<(u64, (u64, (u64, List<Bytes>))), R<((u64, (u64, List<Bytes>)), bool)>>(Ok((v2, true)))
                        }
                        Some(v4) => {
                            Ok::<(u64, (u64, (u64, List<Bytes>))), R<((u64, (u64, List<Bytes>)), bool)>>((v3, v4))
                        }
                    }
                }
            };
            match step {
                Ok(next) => { m28 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f5(v0: u64, v1: List<Adt2>, v2: (u64, (u64, List<Bytes>))) -> R<Option<(u64, (u64, List<Bytes>))>> {
    f17(v1, v0, v2)
}

pub fn f6(v0: u64, v1: u64) -> R<u64> {
    let v2: u64 = {
        let a31 = v1;
        let a32 = 1024u64;
        nat_mul(a31, a32)?
    };
    let a35 = {
        let a33 = v2;
        let a34 = 1024u64;
        nat_add(a33, a34)?
    };
    let a36 = v0;
    if nat_le(a35, a36) {
        Ok(1024u64)
    } else {
        let a37 = v0;
        let a38 = v2;
        Ok(nat_sub(a37, a38))
    }
}

pub fn f7(v0: List<Bytes>) -> Bytes {
    let m40 = {
        let (h39, _) = f18(Fn4::F19, 64u64, v0);
        h39
    };
    match m40.uncons() {
        None => {
            Bytes::lit(&[])
        }
        Some((v1, _)) => {
            v1
        }
    }
}

pub fn f8(v0: u64) -> Bytes {
    let a41 = f20(f21(v0));
    let a42 = f20(f22(v0));
    append_bytes(a41, a42)
}

pub fn f9(v0: Fn1, p46: List<Bytes>, p47: List<Bytes>) -> List<Bytes> {
    let m45 = (p46, p47);
    {
        let mut m45 = m45;
        loop {
            let step = {
                let (v1, v2) = m45;
                let m43 = v2;
                match m43.uncons() {
                    None => {
                        Err::<(List<Bytes>, List<Bytes>), List<Bytes>>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(List<Bytes>, List<Bytes>), List<Bytes>>(({
                            let c44: Fn1 = v0.clone();
                            c44.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m45 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f10(v0: List<Bytes>, v1: Bytes) -> List<Bytes> {
    List::cons(v1, v0)
}

pub fn f11(v0: Adt0) -> R<Result<Adt8, Adt1>> {
    let m48 = v0;
    match m48 {
        Adt0::C0 => {
            Ok(Ok::<Adt8, Adt1>(f23(f24(), 0u32)))
        }
        Adt0::C1(v1) => {
            let a50 = {
                let a49 = v1.clone();
                length_bytes(a49)
            };
            let a51 = 32u64;
            if nat_eq(a50, a51) {
                Ok(Ok::<Adt8, Adt1>(f23(f25(v1)?, f26())))
            } else {
                Ok(Err::<Adt8, Adt1>(Adt1::C0({
                    let a52 = v1;
                    length_bytes(a52)
                })))
            }
        }
        Adt0::C2(v2) => {
            Ok(Ok::<Adt8, Adt1>(f27(v2)?))
        }
    }
}

pub fn f12(v0: Adt8, v1: u64) -> R<Adt5> {
    let v2: Adt3 = match v0.clone() {
        Adt8::C0(h53, _, _, _, _, _, _) => {
            h53
        }
    };
    let v3: u32 = match v0.clone() {
        Adt8::C0(_, h54, _, _, _, _, _) => {
            h54
        }
    };
    let v4: Bytes = match v0.clone() {
        Adt8::C0(_, _, _, h55, _, _, _) => {
            h55
        }
    };
    let v5: (Adt9, List<Adt6>) = f28(v2.clone(), v3, v4.clone(), 0u64, {
        let a56 = v4.clone();
        length_bytes(a56)
    }, match v0.clone() {
        Adt8::C0(_, _, h57, _, _, _, _) => {
            h57
        }
    }, match v0.clone() {
        Adt8::C0(_, _, _, _, _, h58, _) => {
            h58
        }
    })?;
    let v6: (Adt9, (List<Adt6>, u64)) = f29(Fn5::F30(v3, v2.clone()), ({
        let (h59, _) = v5;
        h59
    }, ({
        let (_, h60) = v5;
        h60
    }, {
        let a62 = match v0.clone() {
            Adt8::C0(_, _, _, _, h61, _, _) => {
                h61
            }
        };
        length_list(a62)
    })), match v0.clone() {
        Adt8::C0(_, _, _, _, h63, _, _) => {
            h63
        }
    })?;
    let v7: (u64, List<Adt6>) = f31(Fn6::F32(v6.clone()), {
        let a66 = {
            let a64 = v1;
            let a65 = 63u64;
            nat_add(a64, a65)?
        };
        let a67 = 64u64;
        let a68 = 0u64;
        nat_quot(a66, a67, a68)
    }, (0u64, {
        let (h69, _) = {
            let (_, h70) = v6;
            h70
        };
        h69
    }))?;
    Ok(Adt5::C0(f33(v3), {
        let a75 = {
            let a72 = match v0.clone() {
                Adt8::C0(_, _, h71, _, _, _, _) => {
                    h71
                }
            };
            let a73 = 1024u64;
            nat_mul(a72, a73)?
        };
        let a76 = {
            let a74 = v4;
            length_bytes(a74)
        };
        nat_add(a75, a76)?
    }, v1, v2, v3, match v0 {
        Adt8::C0(_, _, _, _, _, _, h77) => {
            h77
        }
    }, f34({
        let (_, h78) = v7;
        h78
    })))
}

pub fn f13(v0: Fn2, p82: Adt8, p83: List<Bytes>) -> R<Adt8> {
    let m81 = (p82, p83);
    {
        let mut m81 = m81;
        loop {
            let step = {
                let (v1, v2) = m81;
                let m79 = v2;
                match m79.uncons() {
                    None => {
                        Err::<(Adt8, List<Bytes>), R<Adt8>>(Ok(v1))
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt8, List<Bytes>), R<Adt8>>(({
                            let c80: Fn2 = v0.clone();
                            c80.apply(v1, v3)?
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m81 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f14(v0: Adt8, v1: Bytes) -> R<Adt8> {
    f35(v0, v1)
}

pub fn f15(v0: Fn3, p87: List<Adt2>, p88: List<Adt6>) -> List<Adt2> {
    let m86 = (p87, p88);
    {
        let mut m86 = m86;
        loop {
            let step = {
                let (v1, v2) = m86;
                let m84 = v2;
                match m84.uncons() {
                    None => {
                        Err::<(List<Adt2>, List<Adt6>), List<Adt2>>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(List<Adt2>, List<Adt6>), List<Adt2>>(({
                            let c85: Fn3 = v0.clone();
                            c85.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m86 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f16(v0: List<Adt2>, v1: Adt6) -> List<Adt2> {
    let m89 = v1;
    match m89 {
        Adt6::C0(v2) => {
            List::cons(v2, v0)
        }
        Adt6::C1(_, _) => {
            v0
        }
        Adt6::C2(_, _) => {
            v0
        }
    }
}

pub fn f17(v0: List<Adt2>, v1: u64, v2: (u64, (u64, List<Bytes>))) -> R<Option<(u64, (u64, List<Bytes>))>> {
    let v3: u64 = {
        let (h90, _) = v2.clone();
        h90
    };
    let v4: u64 = {
        let (h91, _) = {
            let (_, h92) = v2.clone();
            h92
        };
        h91
    };
    let v5: List<Bytes> = {
        let (_, h93) = {
            let (_, h94) = v2;
            h94
        };
        h93
    };
    let a95 = v4;
    let a96 = 1u64;
    if nat_le(a95, a96) {
        Ok(None::<(u64, (u64, List<Bytes>))>)
    } else {
        let v6: u64 = f36(v4)?;
        let v7: Bytes = {
            let a101 = f37(f38(v0.clone(), v3, v6));
            let a102 = f37(f38(v0, {
                let a97 = v3;
                let a98 = v6;
                nat_add(a97, a98)?
            }, {
                let a99 = v4;
                let a100 = v6;
                nat_sub(a99, a100)
            }));
            append_bytes(a101, a102)
        };
        let a105 = v1;
        let a106 = {
            let a103 = v3;
            let a104 = v6;
            nat_add(a103, a104)?
        };
        if nat_lt(a105, a106) {
            Ok(Some((v3, (v6, List::cons(v7, v5)))))
        } else {
            Ok(Some(({
                let a107 = v3;
                let a108 = v6;
                nat_add(a107, a108)?
            }, ({
                let a109 = v4;
                let a110 = v6;
                nat_sub(a109, a110)
            }, List::cons(v7, v5)))))
        }
    }
}

pub fn f18(v0: Fn4, p115: u64, p116: List<Bytes>) -> (List<Bytes>, bool) {
    let m114 = (p115, p116);
    {
        let mut m114 = m114;
        loop {
            let step = {
                let (v1, v2) = m114;
                let m111 = v1;
                if m111 == 0 {
                    Err::<(u64, List<Bytes>), (List<Bytes>, bool)>((v2, false))
                } else {
                    let v3 = m111 - 1;
                    let m113 = {
                        let c112: Fn4 = v0.clone();
                        c112.apply(v2.clone())
                    };
                    match m113 {
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
                Ok(next) => { m114 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f19(v0: List<Bytes>) -> Option<List<Bytes>> {
    let a118 = {
        let a117 = v0.clone();
        length_list(a117)
    };
    let a119 = 1u64;
    if nat_le(a118, a119) {
        None::<List<Bytes>>
    } else {
        Some(f39(v0))
    }
}

pub fn f20(v0: u32) -> Bytes {
    let a130 = f40({
        let a120 = v0;
        let a121 = 255u32;
        fixed_u32::bit_and(a120, a121)
    });
    let a131 = {
        let a128 = f40({
            let a122 = f41(v0, 8u32);
            let a123 = 255u32;
            fixed_u32::bit_and(a122, a123)
        });
        let a129 = {
            let a126 = f40({
                let a124 = f41(v0, 16u32);
                let a125 = 255u32;
                fixed_u32::bit_and(a124, a125)
            });
            let a127 = f40(f41(v0, 24u32));
            append_bytes(a126, a127)
        };
        append_bytes(a128, a129)
    };
    append_bytes(a130, a131)
}

pub fn f21(v0: u64) -> u32 {
    f42({
        let a132 = v0;
        let a133 = 4294967296u64;
        let a134 = 0u64;
        nat_rem(a132, a133, a134)
    }, 32u64)
}

pub fn f22(v0: u64) -> u32 {
    let a135 = v0;
    let a136 = 4294967296u64;
    if nat_lt(a135, a136) {
        0u32
    } else {
        f42({
            let a137 = v0;
            let a138 = 4294967296u64;
            let a139 = 0u64;
            nat_quot(a137, a138, a139)
        }, 32u64)
    }
}

pub fn f23(v0: Adt3, v1: u32) -> Adt8 {
    Adt8::C0(v0, v1, 0u64, Bytes::lit(&[]), List::<Adt7>::nil(), List::<Adt6>::nil(), List::<Adt6>::nil())
}

pub fn f24() -> Adt3 {
    Adt3::C0(1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, 1359893119u32, 2600822924u32, 528734635u32, 1541459225u32)
}

pub fn f25(v0: Bytes) -> R<Adt3> {
    Ok(f43(f44(v0.clone(), {
        let a140 = v0;
        length_bytes(a140)
    }, 0u64)?))
}

pub fn f26() -> u32 {
    16u32
}

pub fn f27(v0: Bytes) -> R<Adt8> {
    let v1: Adt5 = f45(v0)?;
    Ok(Adt8::C0(f43(f46(v1.clone(), 0u64)), f47(), 0u64, Bytes::lit(&[]), List::<Adt7>::nil(), List::<Adt6>::nil(), match v1 {
        Adt5::C0(_, _, _, _, _, _, h141) => {
            h141
        }
    }))
}

pub fn f28(v0: Adt3, v1: u32, v2: Bytes, v3: u64, v4: u64, v5: u64, v6: List<Adt6>) -> R<(Adt9, List<Adt6>)> {
    let v7: u64 = {
        let a142 = v3;
        let a143 = v4;
        nat_add(a142, a143)?
    };
    let v8: u64 = f48(v4)?;
    let v9: (u64, (Adt3, List<Adt6>)) = f49(Fn7::F50(v1, v3, v5, v0.clone(), v7, v2.clone()), {
        let a144 = v8;
        let a145 = 64u64;
        let a146 = 0u64;
        nat_quot(a144, a145, a146)
    }, (0u64, (v0.clone(), v6)))?;
    Ok((f51(v0, v1, v2, v7, v3, v5, v8, {
        let (h147, _) = {
            let (_, h148) = v9.clone();
            h148
        };
        h147
    }, {
        let a149 = v4;
        let a150 = v8;
        nat_sub(a149, a150)
    }, {
        let a151 = f52();
        let a152 = f53(v8);
        fixed_u32::bit_or(a151, a152)
    })?, {
        let (_, h153) = {
            let (_, h154) = v9;
            h154
        };
        h153
    }))
}

pub fn f29(v0: Fn5, p158: (Adt9, (List<Adt6>, u64)), p159: List<Adt7>) -> R<(Adt9, (List<Adt6>, u64))> {
    let m157 = (p158, p159);
    {
        let mut m157 = m157;
        loop {
            let step = {
                let (v1, v2) = m157;
                let m155 = v2;
                match m155.uncons() {
                    None => {
                        Err::<((Adt9, (List<Adt6>, u64)), List<Adt7>), R<(Adt9, (List<Adt6>, u64))>>(Ok(v1))
                    }
                    Some((v3, v4)) => {
                        Ok::<((Adt9, (List<Adt6>, u64)), List<Adt7>), R<(Adt9, (List<Adt6>, u64))>>(({
                            let c156: Fn5 = v0.clone();
                            c156.apply(v1, v3)?
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m157 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f30(v0: u32, v1: Adt3, v2: (Adt9, (List<Adt6>, u64)), v3: Adt7) -> R<(Adt9, (List<Adt6>, u64))> {
    let v4: Adt2 = f54({
        let (h160, _) = v2.clone();
        h160
    })?;
    Ok((f55(v3.clone(), Adt7::C0(f43(match v4.clone() {
        Adt2::C0(_, _, _, _, _, _, _, _, _, _, _, h161) => {
            h161
        }
    }), {
        let m164 = {
            let (h163, _) = v2.clone();
            h163
        };
        match m164 {
            Adt9::C0(_, h162, _, _, _, _, _, _, _, _) => {
                h162
            }
        }
    }, {
        let m167 = {
            let (h166, _) = v2.clone();
            h166
        };
        match m167 {
            Adt9::C0(_, _, h165, _, _, _, _, _, _, _) => {
                h165
            }
        }
    }), v1, v0)?, (List::cons(Adt6::C2(v3, {
        let (_, h168) = {
            let (_, h169) = v2.clone();
            h169
        };
        h168
    }), List::cons(Adt6::C0(v4), {
        let (h170, _) = {
            let (_, h171) = v2.clone();
            h171
        };
        h170
    })), {
        let a174 = {
            let (_, h172) = {
                let (_, h173) = v2;
                h173
            };
            h172
        };
        let a175 = 1u64;
        nat_sub(a174, a175)
    })))
}

pub fn f31(v0: Fn6, p179: u64, p180: (u64, List<Adt6>)) -> R<(u64, List<Adt6>)> {
    let m178 = (p179, p180);
    {
        let mut m178 = m178;
        loop {
            let step = {
                let (v1, v2) = m178;
                let m176 = v1;
                if m176 == 0 {
                    Err::<(u64, (u64, List<Adt6>)), R<(u64, List<Adt6>)>>(Ok(v2))
                } else {
                    let v3 = m176 - 1;
                    Ok::<(u64, (u64, List<Adt6>)), R<(u64, List<Adt6>)>>((v3, {
                        let c177: Fn6 = v0.clone();
                        c177.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m178 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f32(v0: (Adt9, (List<Adt6>, u64)), v1: (u64, List<Adt6>)) -> R<(u64, List<Adt6>)> {
    Ok(({
        let a182 = {
            let (h181, _) = v1.clone();
            h181
        };
        let a183 = 1u64;
        nat_add(a182, a183)?
    }, List::cons(Adt6::C0(f54(f56({
        let (h184, _) = v0;
        h184
    }, {
        let (h185, _) = v1;
        h185
    }))?), {
        let (_, h186) = v1;
        h186
    })))
}

pub fn f33(v0: u32) -> u64 {
    let a187 = v0;
    let a188 = f26();
    if equal(a187, a188) {
        1u64
    } else {
        let a189 = v0;
        let a190 = f47();
        if equal(a189, a190) {
            2u64
        } else {
            let a191 = v0;
            let a192 = f57();
            if equal(a191, a192) {
                3u64
            } else {
                0u64
            }
        }
    }
}

pub fn f34(v0: List<Adt6>) -> List<Adt6> {
    f58(Fn8::F59, List::<Adt6>::nil(), v0)
}

pub fn f35(v0: Adt8, v1: Bytes) -> R<Adt8> {
    let v2: Bytes = {
        let a194 = match v0.clone() {
            Adt8::C0(_, _, _, h193, _, _, _) => {
                h193
            }
        };
        let a195 = v1;
        append_bytes(a194, a195)
    };
    let v3: u64 = {
        let a196 = v2.clone();
        length_bytes(a196)
    };
    let v4: u64 = {
        let a197 = v3;
        let a198 = 0u64;
        if nat_eq(a197, a198) {
            0u64
        } else {
            let a201 = {
                let a199 = v3;
                let a200 = 1u64;
                nat_sub(a199, a200)
            };
            let a202 = 1024u64;
            let a203 = 0u64;
            nat_quot(a201, a202, a203)
        }
    };
    let v5: u32 = match v0.clone() {
        Adt8::C0(_, h204, _, _, _, _, _) => {
            h204
        }
    };
    let v6: Adt3 = match v0.clone() {
        Adt8::C0(h205, _, _, _, _, _, _) => {
            h205
        }
    };
    let v7: (u64, (List<Adt7>, List<Adt6>)) = f60(Fn9::F61(v5, v2.clone(), v6.clone(), v0.clone()), v4, (0u64, (match v0.clone() {
        Adt8::C0(_, _, _, _, h206, _, _) => {
            h206
        }
    }, match v0.clone() {
        Adt8::C0(_, _, _, _, _, h207, _) => {
            h207
        }
    })))?;
    let v8: u64 = {
        let a208 = v4;
        let a209 = 1024u64;
        nat_mul(a208, a209)?
    };
    Ok(Adt8::C0(v6, v5, {
        let a211 = match v0.clone() {
            Adt8::C0(_, _, h210, _, _, _, _) => {
                h210
            }
        };
        let a212 = v4;
        nat_add(a211, a212)?
    }, {
        let m218 = {
            let a215 = v2;
            let a216 = v8;
            let a217 = {
                let a213 = v3;
                let a214 = v8;
                nat_sub(a213, a214)
            };
            slice_bytes(a215, a216, a217)
        };
        match m218 {
            None => {
                Bytes::lit(&[])
            }
            Some(v9) => {
                v9
            }
        }
    }, {
        let (h219, _) = {
            let (_, h220) = v7.clone();
            h220
        };
        h219
    }, {
        let (_, h221) = {
            let (_, h222) = v7;
            h222
        };
        h221
    }, match v0 {
        Adt8::C0(_, _, _, _, _, _, h223) => {
            h223
        }
    }))
}

pub fn f36(v0: u64) -> R<u64> {
    let (h224, _) = f62(Fn10::F63(v0), 64u64, 1u64)?;
    Ok(h224)
}

pub fn f37(v0: Adt3) -> Bytes {
    let a245 = f20(match v0.clone() {
        Adt3::C0(h225, _, _, _, _, _, _, _) => {
            h225
        }
    });
    let a246 = {
        let a243 = f20(match v0.clone() {
            Adt3::C0(_, h226, _, _, _, _, _, _) => {
                h226
            }
        });
        let a244 = {
            let a241 = f20(match v0.clone() {
                Adt3::C0(_, _, h227, _, _, _, _, _) => {
                    h227
                }
            });
            let a242 = {
                let a239 = f20(match v0.clone() {
                    Adt3::C0(_, _, _, h228, _, _, _, _) => {
                        h228
                    }
                });
                let a240 = {
                    let a237 = f20(match v0.clone() {
                        Adt3::C0(_, _, _, _, h229, _, _, _) => {
                            h229
                        }
                    });
                    let a238 = {
                        let a235 = f20(match v0.clone() {
                            Adt3::C0(_, _, _, _, _, h230, _, _) => {
                                h230
                            }
                        });
                        let a236 = {
                            let a233 = f20(match v0.clone() {
                                Adt3::C0(_, _, _, _, _, _, h231, _) => {
                                    h231
                                }
                            });
                            let a234 = f20(match v0 {
                                Adt3::C0(_, _, _, _, _, _, _, h232) => {
                                    h232
                                }
                            });
                            append_bytes(a233, a234)
                        };
                        append_bytes(a235, a236)
                    };
                    append_bytes(a237, a238)
                };
                append_bytes(a239, a240)
            };
            append_bytes(a241, a242)
        };
        append_bytes(a243, a244)
    };
    append_bytes(a245, a246)
}

pub fn f38(v0: List<Adt2>, v1: u64, v2: u64) -> Adt3 {
    f43(f64(Fn11::F65(v1, v2), Adt4::C0(0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32), v0))
}

pub fn f39(v0: List<Bytes>) -> List<Bytes> {
    let v1: (Option<Bytes>, List<Bytes>) = f66(Fn12::F67, (None::<Bytes>, List::<Bytes>::nil()), v0);
    let v3: List<Bytes> = {
        let m248 = {
            let (h247, _) = v1;
            h247
        };
        match m248 {
            None => {
                let (_, h250) = v1;
                h250
            }
            Some(v2) => {
                List::cons(v2, {
                    let (_, h249) = v1;
                    h249
                })
            }
        }
    };
    f9(Fn1::F68, List::<Bytes>::nil(), v3)
}

pub fn f40(v0: u32) -> Bytes {
    let a253 = {
        let a251 = v0;
        let a252 = 128u32;
        fixed_u32::bit_and(a251, a252)
    };
    let a254 = 0u32;
    if equal(a253, a254) {
        let a257 = {
            let a255 = v0;
            let a256 = 64u32;
            fixed_u32::bit_and(a255, a256)
        };
        let a258 = 0u32;
        if equal(a257, a258) {
            let a261 = {
                let a259 = v0;
                let a260 = 32u32;
                fixed_u32::bit_and(a259, a260)
            };
            let a262 = 0u32;
            if equal(a261, a262) {
                let a265 = {
                    let a263 = v0;
                    let a264 = 16u32;
                    fixed_u32::bit_and(a263, a264)
                };
                let a266 = 0u32;
                if equal(a265, a266) {
                    let a269 = {
                        let a267 = v0;
                        let a268 = 8u32;
                        fixed_u32::bit_and(a267, a268)
                    };
                    let a270 = 0u32;
                    if equal(a269, a270) {
                        let a273 = {
                            let a271 = v0;
                            let a272 = 4u32;
                            fixed_u32::bit_and(a271, a272)
                        };
                        let a274 = 0u32;
                        if equal(a273, a274) {
                            let a277 = {
                                let a275 = v0;
                                let a276 = 2u32;
                                fixed_u32::bit_and(a275, a276)
                            };
                            let a278 = 0u32;
                            if equal(a277, a278) {
                                let a281 = {
                                    let a279 = v0;
                                    let a280 = 1u32;
                                    fixed_u32::bit_and(a279, a280)
                                };
                                let a282 = 0u32;
                                if equal(a281, a282) {
                                    Bytes::lit(&[0x00])
                                } else {
                                    Bytes::lit(&[0x01])
                                }
                            } else {
                                let a285 = {
                                    let a283 = v0;
                                    let a284 = 1u32;
                                    fixed_u32::bit_and(a283, a284)
                                };
                                let a286 = 0u32;
                                if equal(a285, a286) {
                                    Bytes::lit(&[0x02])
                                } else {
                                    Bytes::lit(&[0x03])
                                }
                            }
                        } else {
                            let a289 = {
                                let a287 = v0;
                                let a288 = 2u32;
                                fixed_u32::bit_and(a287, a288)
                            };
                            let a290 = 0u32;
                            if equal(a289, a290) {
                                let a293 = {
                                    let a291 = v0;
                                    let a292 = 1u32;
                                    fixed_u32::bit_and(a291, a292)
                                };
                                let a294 = 0u32;
                                if equal(a293, a294) {
                                    Bytes::lit(&[0x04])
                                } else {
                                    Bytes::lit(&[0x05])
                                }
                            } else {
                                let a297 = {
                                    let a295 = v0;
                                    let a296 = 1u32;
                                    fixed_u32::bit_and(a295, a296)
                                };
                                let a298 = 0u32;
                                if equal(a297, a298) {
                                    Bytes::lit(&[0x06])
                                } else {
                                    Bytes::lit(&[0x07])
                                }
                            }
                        }
                    } else {
                        let a301 = {
                            let a299 = v0;
                            let a300 = 4u32;
                            fixed_u32::bit_and(a299, a300)
                        };
                        let a302 = 0u32;
                        if equal(a301, a302) {
                            let a305 = {
                                let a303 = v0;
                                let a304 = 2u32;
                                fixed_u32::bit_and(a303, a304)
                            };
                            let a306 = 0u32;
                            if equal(a305, a306) {
                                let a309 = {
                                    let a307 = v0;
                                    let a308 = 1u32;
                                    fixed_u32::bit_and(a307, a308)
                                };
                                let a310 = 0u32;
                                if equal(a309, a310) {
                                    Bytes::lit(&[0x08])
                                } else {
                                    Bytes::lit(&[0x09])
                                }
                            } else {
                                let a313 = {
                                    let a311 = v0;
                                    let a312 = 1u32;
                                    fixed_u32::bit_and(a311, a312)
                                };
                                let a314 = 0u32;
                                if equal(a313, a314) {
                                    Bytes::lit(&[0x0a])
                                } else {
                                    Bytes::lit(&[0x0b])
                                }
                            }
                        } else {
                            let a317 = {
                                let a315 = v0;
                                let a316 = 2u32;
                                fixed_u32::bit_and(a315, a316)
                            };
                            let a318 = 0u32;
                            if equal(a317, a318) {
                                let a321 = {
                                    let a319 = v0;
                                    let a320 = 1u32;
                                    fixed_u32::bit_and(a319, a320)
                                };
                                let a322 = 0u32;
                                if equal(a321, a322) {
                                    Bytes::lit(&[0x0c])
                                } else {
                                    Bytes::lit(&[0x0d])
                                }
                            } else {
                                let a325 = {
                                    let a323 = v0;
                                    let a324 = 1u32;
                                    fixed_u32::bit_and(a323, a324)
                                };
                                let a326 = 0u32;
                                if equal(a325, a326) {
                                    Bytes::lit(&[0x0e])
                                } else {
                                    Bytes::lit(&[0x0f])
                                }
                            }
                        }
                    }
                } else {
                    let a329 = {
                        let a327 = v0;
                        let a328 = 8u32;
                        fixed_u32::bit_and(a327, a328)
                    };
                    let a330 = 0u32;
                    if equal(a329, a330) {
                        let a333 = {
                            let a331 = v0;
                            let a332 = 4u32;
                            fixed_u32::bit_and(a331, a332)
                        };
                        let a334 = 0u32;
                        if equal(a333, a334) {
                            let a337 = {
                                let a335 = v0;
                                let a336 = 2u32;
                                fixed_u32::bit_and(a335, a336)
                            };
                            let a338 = 0u32;
                            if equal(a337, a338) {
                                let a341 = {
                                    let a339 = v0;
                                    let a340 = 1u32;
                                    fixed_u32::bit_and(a339, a340)
                                };
                                let a342 = 0u32;
                                if equal(a341, a342) {
                                    Bytes::lit(&[0x10])
                                } else {
                                    Bytes::lit(&[0x11])
                                }
                            } else {
                                let a345 = {
                                    let a343 = v0;
                                    let a344 = 1u32;
                                    fixed_u32::bit_and(a343, a344)
                                };
                                let a346 = 0u32;
                                if equal(a345, a346) {
                                    Bytes::lit(&[0x12])
                                } else {
                                    Bytes::lit(&[0x13])
                                }
                            }
                        } else {
                            let a349 = {
                                let a347 = v0;
                                let a348 = 2u32;
                                fixed_u32::bit_and(a347, a348)
                            };
                            let a350 = 0u32;
                            if equal(a349, a350) {
                                let a353 = {
                                    let a351 = v0;
                                    let a352 = 1u32;
                                    fixed_u32::bit_and(a351, a352)
                                };
                                let a354 = 0u32;
                                if equal(a353, a354) {
                                    Bytes::lit(&[0x14])
                                } else {
                                    Bytes::lit(&[0x15])
                                }
                            } else {
                                let a357 = {
                                    let a355 = v0;
                                    let a356 = 1u32;
                                    fixed_u32::bit_and(a355, a356)
                                };
                                let a358 = 0u32;
                                if equal(a357, a358) {
                                    Bytes::lit(&[0x16])
                                } else {
                                    Bytes::lit(&[0x17])
                                }
                            }
                        }
                    } else {
                        let a361 = {
                            let a359 = v0;
                            let a360 = 4u32;
                            fixed_u32::bit_and(a359, a360)
                        };
                        let a362 = 0u32;
                        if equal(a361, a362) {
                            let a365 = {
                                let a363 = v0;
                                let a364 = 2u32;
                                fixed_u32::bit_and(a363, a364)
                            };
                            let a366 = 0u32;
                            if equal(a365, a366) {
                                let a369 = {
                                    let a367 = v0;
                                    let a368 = 1u32;
                                    fixed_u32::bit_and(a367, a368)
                                };
                                let a370 = 0u32;
                                if equal(a369, a370) {
                                    Bytes::lit(&[0x18])
                                } else {
                                    Bytes::lit(&[0x19])
                                }
                            } else {
                                let a373 = {
                                    let a371 = v0;
                                    let a372 = 1u32;
                                    fixed_u32::bit_and(a371, a372)
                                };
                                let a374 = 0u32;
                                if equal(a373, a374) {
                                    Bytes::lit(&[0x1a])
                                } else {
                                    Bytes::lit(&[0x1b])
                                }
                            }
                        } else {
                            let a377 = {
                                let a375 = v0;
                                let a376 = 2u32;
                                fixed_u32::bit_and(a375, a376)
                            };
                            let a378 = 0u32;
                            if equal(a377, a378) {
                                let a381 = {
                                    let a379 = v0;
                                    let a380 = 1u32;
                                    fixed_u32::bit_and(a379, a380)
                                };
                                let a382 = 0u32;
                                if equal(a381, a382) {
                                    Bytes::lit(&[0x1c])
                                } else {
                                    Bytes::lit(&[0x1d])
                                }
                            } else {
                                let a385 = {
                                    let a383 = v0;
                                    let a384 = 1u32;
                                    fixed_u32::bit_and(a383, a384)
                                };
                                let a386 = 0u32;
                                if equal(a385, a386) {
                                    Bytes::lit(&[0x1e])
                                } else {
                                    Bytes::lit(&[0x1f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a389 = {
                    let a387 = v0;
                    let a388 = 16u32;
                    fixed_u32::bit_and(a387, a388)
                };
                let a390 = 0u32;
                if equal(a389, a390) {
                    let a393 = {
                        let a391 = v0;
                        let a392 = 8u32;
                        fixed_u32::bit_and(a391, a392)
                    };
                    let a394 = 0u32;
                    if equal(a393, a394) {
                        let a397 = {
                            let a395 = v0;
                            let a396 = 4u32;
                            fixed_u32::bit_and(a395, a396)
                        };
                        let a398 = 0u32;
                        if equal(a397, a398) {
                            let a401 = {
                                let a399 = v0;
                                let a400 = 2u32;
                                fixed_u32::bit_and(a399, a400)
                            };
                            let a402 = 0u32;
                            if equal(a401, a402) {
                                let a405 = {
                                    let a403 = v0;
                                    let a404 = 1u32;
                                    fixed_u32::bit_and(a403, a404)
                                };
                                let a406 = 0u32;
                                if equal(a405, a406) {
                                    Bytes::lit(&[0x20])
                                } else {
                                    Bytes::lit(&[0x21])
                                }
                            } else {
                                let a409 = {
                                    let a407 = v0;
                                    let a408 = 1u32;
                                    fixed_u32::bit_and(a407, a408)
                                };
                                let a410 = 0u32;
                                if equal(a409, a410) {
                                    Bytes::lit(&[0x22])
                                } else {
                                    Bytes::lit(&[0x23])
                                }
                            }
                        } else {
                            let a413 = {
                                let a411 = v0;
                                let a412 = 2u32;
                                fixed_u32::bit_and(a411, a412)
                            };
                            let a414 = 0u32;
                            if equal(a413, a414) {
                                let a417 = {
                                    let a415 = v0;
                                    let a416 = 1u32;
                                    fixed_u32::bit_and(a415, a416)
                                };
                                let a418 = 0u32;
                                if equal(a417, a418) {
                                    Bytes::lit(&[0x24])
                                } else {
                                    Bytes::lit(&[0x25])
                                }
                            } else {
                                let a421 = {
                                    let a419 = v0;
                                    let a420 = 1u32;
                                    fixed_u32::bit_and(a419, a420)
                                };
                                let a422 = 0u32;
                                if equal(a421, a422) {
                                    Bytes::lit(&[0x26])
                                } else {
                                    Bytes::lit(&[0x27])
                                }
                            }
                        }
                    } else {
                        let a425 = {
                            let a423 = v0;
                            let a424 = 4u32;
                            fixed_u32::bit_and(a423, a424)
                        };
                        let a426 = 0u32;
                        if equal(a425, a426) {
                            let a429 = {
                                let a427 = v0;
                                let a428 = 2u32;
                                fixed_u32::bit_and(a427, a428)
                            };
                            let a430 = 0u32;
                            if equal(a429, a430) {
                                let a433 = {
                                    let a431 = v0;
                                    let a432 = 1u32;
                                    fixed_u32::bit_and(a431, a432)
                                };
                                let a434 = 0u32;
                                if equal(a433, a434) {
                                    Bytes::lit(&[0x28])
                                } else {
                                    Bytes::lit(&[0x29])
                                }
                            } else {
                                let a437 = {
                                    let a435 = v0;
                                    let a436 = 1u32;
                                    fixed_u32::bit_and(a435, a436)
                                };
                                let a438 = 0u32;
                                if equal(a437, a438) {
                                    Bytes::lit(&[0x2a])
                                } else {
                                    Bytes::lit(&[0x2b])
                                }
                            }
                        } else {
                            let a441 = {
                                let a439 = v0;
                                let a440 = 2u32;
                                fixed_u32::bit_and(a439, a440)
                            };
                            let a442 = 0u32;
                            if equal(a441, a442) {
                                let a445 = {
                                    let a443 = v0;
                                    let a444 = 1u32;
                                    fixed_u32::bit_and(a443, a444)
                                };
                                let a446 = 0u32;
                                if equal(a445, a446) {
                                    Bytes::lit(&[0x2c])
                                } else {
                                    Bytes::lit(&[0x2d])
                                }
                            } else {
                                let a449 = {
                                    let a447 = v0;
                                    let a448 = 1u32;
                                    fixed_u32::bit_and(a447, a448)
                                };
                                let a450 = 0u32;
                                if equal(a449, a450) {
                                    Bytes::lit(&[0x2e])
                                } else {
                                    Bytes::lit(&[0x2f])
                                }
                            }
                        }
                    }
                } else {
                    let a453 = {
                        let a451 = v0;
                        let a452 = 8u32;
                        fixed_u32::bit_and(a451, a452)
                    };
                    let a454 = 0u32;
                    if equal(a453, a454) {
                        let a457 = {
                            let a455 = v0;
                            let a456 = 4u32;
                            fixed_u32::bit_and(a455, a456)
                        };
                        let a458 = 0u32;
                        if equal(a457, a458) {
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
                                    Bytes::lit(&[0x30])
                                } else {
                                    Bytes::lit(&[0x31])
                                }
                            } else {
                                let a469 = {
                                    let a467 = v0;
                                    let a468 = 1u32;
                                    fixed_u32::bit_and(a467, a468)
                                };
                                let a470 = 0u32;
                                if equal(a469, a470) {
                                    Bytes::lit(&[0x32])
                                } else {
                                    Bytes::lit(&[0x33])
                                }
                            }
                        } else {
                            let a473 = {
                                let a471 = v0;
                                let a472 = 2u32;
                                fixed_u32::bit_and(a471, a472)
                            };
                            let a474 = 0u32;
                            if equal(a473, a474) {
                                let a477 = {
                                    let a475 = v0;
                                    let a476 = 1u32;
                                    fixed_u32::bit_and(a475, a476)
                                };
                                let a478 = 0u32;
                                if equal(a477, a478) {
                                    Bytes::lit(&[0x34])
                                } else {
                                    Bytes::lit(&[0x35])
                                }
                            } else {
                                let a481 = {
                                    let a479 = v0;
                                    let a480 = 1u32;
                                    fixed_u32::bit_and(a479, a480)
                                };
                                let a482 = 0u32;
                                if equal(a481, a482) {
                                    Bytes::lit(&[0x36])
                                } else {
                                    Bytes::lit(&[0x37])
                                }
                            }
                        }
                    } else {
                        let a485 = {
                            let a483 = v0;
                            let a484 = 4u32;
                            fixed_u32::bit_and(a483, a484)
                        };
                        let a486 = 0u32;
                        if equal(a485, a486) {
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
                                    Bytes::lit(&[0x38])
                                } else {
                                    Bytes::lit(&[0x39])
                                }
                            } else {
                                let a497 = {
                                    let a495 = v0;
                                    let a496 = 1u32;
                                    fixed_u32::bit_and(a495, a496)
                                };
                                let a498 = 0u32;
                                if equal(a497, a498) {
                                    Bytes::lit(&[0x3a])
                                } else {
                                    Bytes::lit(&[0x3b])
                                }
                            }
                        } else {
                            let a501 = {
                                let a499 = v0;
                                let a500 = 2u32;
                                fixed_u32::bit_and(a499, a500)
                            };
                            let a502 = 0u32;
                            if equal(a501, a502) {
                                let a505 = {
                                    let a503 = v0;
                                    let a504 = 1u32;
                                    fixed_u32::bit_and(a503, a504)
                                };
                                let a506 = 0u32;
                                if equal(a505, a506) {
                                    Bytes::lit(&[0x3c])
                                } else {
                                    Bytes::lit(&[0x3d])
                                }
                            } else {
                                let a509 = {
                                    let a507 = v0;
                                    let a508 = 1u32;
                                    fixed_u32::bit_and(a507, a508)
                                };
                                let a510 = 0u32;
                                if equal(a509, a510) {
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
            let a513 = {
                let a511 = v0;
                let a512 = 32u32;
                fixed_u32::bit_and(a511, a512)
            };
            let a514 = 0u32;
            if equal(a513, a514) {
                let a517 = {
                    let a515 = v0;
                    let a516 = 16u32;
                    fixed_u32::bit_and(a515, a516)
                };
                let a518 = 0u32;
                if equal(a517, a518) {
                    let a521 = {
                        let a519 = v0;
                        let a520 = 8u32;
                        fixed_u32::bit_and(a519, a520)
                    };
                    let a522 = 0u32;
                    if equal(a521, a522) {
                        let a525 = {
                            let a523 = v0;
                            let a524 = 4u32;
                            fixed_u32::bit_and(a523, a524)
                        };
                        let a526 = 0u32;
                        if equal(a525, a526) {
                            let a529 = {
                                let a527 = v0;
                                let a528 = 2u32;
                                fixed_u32::bit_and(a527, a528)
                            };
                            let a530 = 0u32;
                            if equal(a529, a530) {
                                let a533 = {
                                    let a531 = v0;
                                    let a532 = 1u32;
                                    fixed_u32::bit_and(a531, a532)
                                };
                                let a534 = 0u32;
                                if equal(a533, a534) {
                                    Bytes::lit(&[0x40])
                                } else {
                                    Bytes::lit(&[0x41])
                                }
                            } else {
                                let a537 = {
                                    let a535 = v0;
                                    let a536 = 1u32;
                                    fixed_u32::bit_and(a535, a536)
                                };
                                let a538 = 0u32;
                                if equal(a537, a538) {
                                    Bytes::lit(&[0x42])
                                } else {
                                    Bytes::lit(&[0x43])
                                }
                            }
                        } else {
                            let a541 = {
                                let a539 = v0;
                                let a540 = 2u32;
                                fixed_u32::bit_and(a539, a540)
                            };
                            let a542 = 0u32;
                            if equal(a541, a542) {
                                let a545 = {
                                    let a543 = v0;
                                    let a544 = 1u32;
                                    fixed_u32::bit_and(a543, a544)
                                };
                                let a546 = 0u32;
                                if equal(a545, a546) {
                                    Bytes::lit(&[0x44])
                                } else {
                                    Bytes::lit(&[0x45])
                                }
                            } else {
                                let a549 = {
                                    let a547 = v0;
                                    let a548 = 1u32;
                                    fixed_u32::bit_and(a547, a548)
                                };
                                let a550 = 0u32;
                                if equal(a549, a550) {
                                    Bytes::lit(&[0x46])
                                } else {
                                    Bytes::lit(&[0x47])
                                }
                            }
                        }
                    } else {
                        let a553 = {
                            let a551 = v0;
                            let a552 = 4u32;
                            fixed_u32::bit_and(a551, a552)
                        };
                        let a554 = 0u32;
                        if equal(a553, a554) {
                            let a557 = {
                                let a555 = v0;
                                let a556 = 2u32;
                                fixed_u32::bit_and(a555, a556)
                            };
                            let a558 = 0u32;
                            if equal(a557, a558) {
                                let a561 = {
                                    let a559 = v0;
                                    let a560 = 1u32;
                                    fixed_u32::bit_and(a559, a560)
                                };
                                let a562 = 0u32;
                                if equal(a561, a562) {
                                    Bytes::lit(&[0x48])
                                } else {
                                    Bytes::lit(&[0x49])
                                }
                            } else {
                                let a565 = {
                                    let a563 = v0;
                                    let a564 = 1u32;
                                    fixed_u32::bit_and(a563, a564)
                                };
                                let a566 = 0u32;
                                if equal(a565, a566) {
                                    Bytes::lit(&[0x4a])
                                } else {
                                    Bytes::lit(&[0x4b])
                                }
                            }
                        } else {
                            let a569 = {
                                let a567 = v0;
                                let a568 = 2u32;
                                fixed_u32::bit_and(a567, a568)
                            };
                            let a570 = 0u32;
                            if equal(a569, a570) {
                                let a573 = {
                                    let a571 = v0;
                                    let a572 = 1u32;
                                    fixed_u32::bit_and(a571, a572)
                                };
                                let a574 = 0u32;
                                if equal(a573, a574) {
                                    Bytes::lit(&[0x4c])
                                } else {
                                    Bytes::lit(&[0x4d])
                                }
                            } else {
                                let a577 = {
                                    let a575 = v0;
                                    let a576 = 1u32;
                                    fixed_u32::bit_and(a575, a576)
                                };
                                let a578 = 0u32;
                                if equal(a577, a578) {
                                    Bytes::lit(&[0x4e])
                                } else {
                                    Bytes::lit(&[0x4f])
                                }
                            }
                        }
                    }
                } else {
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
                                    Bytes::lit(&[0x50])
                                } else {
                                    Bytes::lit(&[0x51])
                                }
                            } else {
                                let a597 = {
                                    let a595 = v0;
                                    let a596 = 1u32;
                                    fixed_u32::bit_and(a595, a596)
                                };
                                let a598 = 0u32;
                                if equal(a597, a598) {
                                    Bytes::lit(&[0x52])
                                } else {
                                    Bytes::lit(&[0x53])
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
                                    Bytes::lit(&[0x54])
                                } else {
                                    Bytes::lit(&[0x55])
                                }
                            } else {
                                let a609 = {
                                    let a607 = v0;
                                    let a608 = 1u32;
                                    fixed_u32::bit_and(a607, a608)
                                };
                                let a610 = 0u32;
                                if equal(a609, a610) {
                                    Bytes::lit(&[0x56])
                                } else {
                                    Bytes::lit(&[0x57])
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
                                    Bytes::lit(&[0x58])
                                } else {
                                    Bytes::lit(&[0x59])
                                }
                            } else {
                                let a625 = {
                                    let a623 = v0;
                                    let a624 = 1u32;
                                    fixed_u32::bit_and(a623, a624)
                                };
                                let a626 = 0u32;
                                if equal(a625, a626) {
                                    Bytes::lit(&[0x5a])
                                } else {
                                    Bytes::lit(&[0x5b])
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
                                    Bytes::lit(&[0x5c])
                                } else {
                                    Bytes::lit(&[0x5d])
                                }
                            } else {
                                let a637 = {
                                    let a635 = v0;
                                    let a636 = 1u32;
                                    fixed_u32::bit_and(a635, a636)
                                };
                                let a638 = 0u32;
                                if equal(a637, a638) {
                                    Bytes::lit(&[0x5e])
                                } else {
                                    Bytes::lit(&[0x5f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a641 = {
                    let a639 = v0;
                    let a640 = 16u32;
                    fixed_u32::bit_and(a639, a640)
                };
                let a642 = 0u32;
                if equal(a641, a642) {
                    let a645 = {
                        let a643 = v0;
                        let a644 = 8u32;
                        fixed_u32::bit_and(a643, a644)
                    };
                    let a646 = 0u32;
                    if equal(a645, a646) {
                        let a649 = {
                            let a647 = v0;
                            let a648 = 4u32;
                            fixed_u32::bit_and(a647, a648)
                        };
                        let a650 = 0u32;
                        if equal(a649, a650) {
                            let a653 = {
                                let a651 = v0;
                                let a652 = 2u32;
                                fixed_u32::bit_and(a651, a652)
                            };
                            let a654 = 0u32;
                            if equal(a653, a654) {
                                let a657 = {
                                    let a655 = v0;
                                    let a656 = 1u32;
                                    fixed_u32::bit_and(a655, a656)
                                };
                                let a658 = 0u32;
                                if equal(a657, a658) {
                                    Bytes::lit(&[0x60])
                                } else {
                                    Bytes::lit(&[0x61])
                                }
                            } else {
                                let a661 = {
                                    let a659 = v0;
                                    let a660 = 1u32;
                                    fixed_u32::bit_and(a659, a660)
                                };
                                let a662 = 0u32;
                                if equal(a661, a662) {
                                    Bytes::lit(&[0x62])
                                } else {
                                    Bytes::lit(&[0x63])
                                }
                            }
                        } else {
                            let a665 = {
                                let a663 = v0;
                                let a664 = 2u32;
                                fixed_u32::bit_and(a663, a664)
                            };
                            let a666 = 0u32;
                            if equal(a665, a666) {
                                let a669 = {
                                    let a667 = v0;
                                    let a668 = 1u32;
                                    fixed_u32::bit_and(a667, a668)
                                };
                                let a670 = 0u32;
                                if equal(a669, a670) {
                                    Bytes::lit(&[0x64])
                                } else {
                                    Bytes::lit(&[0x65])
                                }
                            } else {
                                let a673 = {
                                    let a671 = v0;
                                    let a672 = 1u32;
                                    fixed_u32::bit_and(a671, a672)
                                };
                                let a674 = 0u32;
                                if equal(a673, a674) {
                                    Bytes::lit(&[0x66])
                                } else {
                                    Bytes::lit(&[0x67])
                                }
                            }
                        }
                    } else {
                        let a677 = {
                            let a675 = v0;
                            let a676 = 4u32;
                            fixed_u32::bit_and(a675, a676)
                        };
                        let a678 = 0u32;
                        if equal(a677, a678) {
                            let a681 = {
                                let a679 = v0;
                                let a680 = 2u32;
                                fixed_u32::bit_and(a679, a680)
                            };
                            let a682 = 0u32;
                            if equal(a681, a682) {
                                let a685 = {
                                    let a683 = v0;
                                    let a684 = 1u32;
                                    fixed_u32::bit_and(a683, a684)
                                };
                                let a686 = 0u32;
                                if equal(a685, a686) {
                                    Bytes::lit(&[0x68])
                                } else {
                                    Bytes::lit(&[0x69])
                                }
                            } else {
                                let a689 = {
                                    let a687 = v0;
                                    let a688 = 1u32;
                                    fixed_u32::bit_and(a687, a688)
                                };
                                let a690 = 0u32;
                                if equal(a689, a690) {
                                    Bytes::lit(&[0x6a])
                                } else {
                                    Bytes::lit(&[0x6b])
                                }
                            }
                        } else {
                            let a693 = {
                                let a691 = v0;
                                let a692 = 2u32;
                                fixed_u32::bit_and(a691, a692)
                            };
                            let a694 = 0u32;
                            if equal(a693, a694) {
                                let a697 = {
                                    let a695 = v0;
                                    let a696 = 1u32;
                                    fixed_u32::bit_and(a695, a696)
                                };
                                let a698 = 0u32;
                                if equal(a697, a698) {
                                    Bytes::lit(&[0x6c])
                                } else {
                                    Bytes::lit(&[0x6d])
                                }
                            } else {
                                let a701 = {
                                    let a699 = v0;
                                    let a700 = 1u32;
                                    fixed_u32::bit_and(a699, a700)
                                };
                                let a702 = 0u32;
                                if equal(a701, a702) {
                                    Bytes::lit(&[0x6e])
                                } else {
                                    Bytes::lit(&[0x6f])
                                }
                            }
                        }
                    }
                } else {
                    let a705 = {
                        let a703 = v0;
                        let a704 = 8u32;
                        fixed_u32::bit_and(a703, a704)
                    };
                    let a706 = 0u32;
                    if equal(a705, a706) {
                        let a709 = {
                            let a707 = v0;
                            let a708 = 4u32;
                            fixed_u32::bit_and(a707, a708)
                        };
                        let a710 = 0u32;
                        if equal(a709, a710) {
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
                                    Bytes::lit(&[0x70])
                                } else {
                                    Bytes::lit(&[0x71])
                                }
                            } else {
                                let a721 = {
                                    let a719 = v0;
                                    let a720 = 1u32;
                                    fixed_u32::bit_and(a719, a720)
                                };
                                let a722 = 0u32;
                                if equal(a721, a722) {
                                    Bytes::lit(&[0x72])
                                } else {
                                    Bytes::lit(&[0x73])
                                }
                            }
                        } else {
                            let a725 = {
                                let a723 = v0;
                                let a724 = 2u32;
                                fixed_u32::bit_and(a723, a724)
                            };
                            let a726 = 0u32;
                            if equal(a725, a726) {
                                let a729 = {
                                    let a727 = v0;
                                    let a728 = 1u32;
                                    fixed_u32::bit_and(a727, a728)
                                };
                                let a730 = 0u32;
                                if equal(a729, a730) {
                                    Bytes::lit(&[0x74])
                                } else {
                                    Bytes::lit(&[0x75])
                                }
                            } else {
                                let a733 = {
                                    let a731 = v0;
                                    let a732 = 1u32;
                                    fixed_u32::bit_and(a731, a732)
                                };
                                let a734 = 0u32;
                                if equal(a733, a734) {
                                    Bytes::lit(&[0x76])
                                } else {
                                    Bytes::lit(&[0x77])
                                }
                            }
                        }
                    } else {
                        let a737 = {
                            let a735 = v0;
                            let a736 = 4u32;
                            fixed_u32::bit_and(a735, a736)
                        };
                        let a738 = 0u32;
                        if equal(a737, a738) {
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
                                    Bytes::lit(&[0x78])
                                } else {
                                    Bytes::lit(&[0x79])
                                }
                            } else {
                                let a749 = {
                                    let a747 = v0;
                                    let a748 = 1u32;
                                    fixed_u32::bit_and(a747, a748)
                                };
                                let a750 = 0u32;
                                if equal(a749, a750) {
                                    Bytes::lit(&[0x7a])
                                } else {
                                    Bytes::lit(&[0x7b])
                                }
                            }
                        } else {
                            let a753 = {
                                let a751 = v0;
                                let a752 = 2u32;
                                fixed_u32::bit_and(a751, a752)
                            };
                            let a754 = 0u32;
                            if equal(a753, a754) {
                                let a757 = {
                                    let a755 = v0;
                                    let a756 = 1u32;
                                    fixed_u32::bit_and(a755, a756)
                                };
                                let a758 = 0u32;
                                if equal(a757, a758) {
                                    Bytes::lit(&[0x7c])
                                } else {
                                    Bytes::lit(&[0x7d])
                                }
                            } else {
                                let a761 = {
                                    let a759 = v0;
                                    let a760 = 1u32;
                                    fixed_u32::bit_and(a759, a760)
                                };
                                let a762 = 0u32;
                                if equal(a761, a762) {
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
        let a765 = {
            let a763 = v0;
            let a764 = 64u32;
            fixed_u32::bit_and(a763, a764)
        };
        let a766 = 0u32;
        if equal(a765, a766) {
            let a769 = {
                let a767 = v0;
                let a768 = 32u32;
                fixed_u32::bit_and(a767, a768)
            };
            let a770 = 0u32;
            if equal(a769, a770) {
                let a773 = {
                    let a771 = v0;
                    let a772 = 16u32;
                    fixed_u32::bit_and(a771, a772)
                };
                let a774 = 0u32;
                if equal(a773, a774) {
                    let a777 = {
                        let a775 = v0;
                        let a776 = 8u32;
                        fixed_u32::bit_and(a775, a776)
                    };
                    let a778 = 0u32;
                    if equal(a777, a778) {
                        let a781 = {
                            let a779 = v0;
                            let a780 = 4u32;
                            fixed_u32::bit_and(a779, a780)
                        };
                        let a782 = 0u32;
                        if equal(a781, a782) {
                            let a785 = {
                                let a783 = v0;
                                let a784 = 2u32;
                                fixed_u32::bit_and(a783, a784)
                            };
                            let a786 = 0u32;
                            if equal(a785, a786) {
                                let a789 = {
                                    let a787 = v0;
                                    let a788 = 1u32;
                                    fixed_u32::bit_and(a787, a788)
                                };
                                let a790 = 0u32;
                                if equal(a789, a790) {
                                    Bytes::lit(&[0x80])
                                } else {
                                    Bytes::lit(&[0x81])
                                }
                            } else {
                                let a793 = {
                                    let a791 = v0;
                                    let a792 = 1u32;
                                    fixed_u32::bit_and(a791, a792)
                                };
                                let a794 = 0u32;
                                if equal(a793, a794) {
                                    Bytes::lit(&[0x82])
                                } else {
                                    Bytes::lit(&[0x83])
                                }
                            }
                        } else {
                            let a797 = {
                                let a795 = v0;
                                let a796 = 2u32;
                                fixed_u32::bit_and(a795, a796)
                            };
                            let a798 = 0u32;
                            if equal(a797, a798) {
                                let a801 = {
                                    let a799 = v0;
                                    let a800 = 1u32;
                                    fixed_u32::bit_and(a799, a800)
                                };
                                let a802 = 0u32;
                                if equal(a801, a802) {
                                    Bytes::lit(&[0x84])
                                } else {
                                    Bytes::lit(&[0x85])
                                }
                            } else {
                                let a805 = {
                                    let a803 = v0;
                                    let a804 = 1u32;
                                    fixed_u32::bit_and(a803, a804)
                                };
                                let a806 = 0u32;
                                if equal(a805, a806) {
                                    Bytes::lit(&[0x86])
                                } else {
                                    Bytes::lit(&[0x87])
                                }
                            }
                        }
                    } else {
                        let a809 = {
                            let a807 = v0;
                            let a808 = 4u32;
                            fixed_u32::bit_and(a807, a808)
                        };
                        let a810 = 0u32;
                        if equal(a809, a810) {
                            let a813 = {
                                let a811 = v0;
                                let a812 = 2u32;
                                fixed_u32::bit_and(a811, a812)
                            };
                            let a814 = 0u32;
                            if equal(a813, a814) {
                                let a817 = {
                                    let a815 = v0;
                                    let a816 = 1u32;
                                    fixed_u32::bit_and(a815, a816)
                                };
                                let a818 = 0u32;
                                if equal(a817, a818) {
                                    Bytes::lit(&[0x88])
                                } else {
                                    Bytes::lit(&[0x89])
                                }
                            } else {
                                let a821 = {
                                    let a819 = v0;
                                    let a820 = 1u32;
                                    fixed_u32::bit_and(a819, a820)
                                };
                                let a822 = 0u32;
                                if equal(a821, a822) {
                                    Bytes::lit(&[0x8a])
                                } else {
                                    Bytes::lit(&[0x8b])
                                }
                            }
                        } else {
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
                                    Bytes::lit(&[0x8c])
                                } else {
                                    Bytes::lit(&[0x8d])
                                }
                            } else {
                                let a833 = {
                                    let a831 = v0;
                                    let a832 = 1u32;
                                    fixed_u32::bit_and(a831, a832)
                                };
                                let a834 = 0u32;
                                if equal(a833, a834) {
                                    Bytes::lit(&[0x8e])
                                } else {
                                    Bytes::lit(&[0x8f])
                                }
                            }
                        }
                    }
                } else {
                    let a837 = {
                        let a835 = v0;
                        let a836 = 8u32;
                        fixed_u32::bit_and(a835, a836)
                    };
                    let a838 = 0u32;
                    if equal(a837, a838) {
                        let a841 = {
                            let a839 = v0;
                            let a840 = 4u32;
                            fixed_u32::bit_and(a839, a840)
                        };
                        let a842 = 0u32;
                        if equal(a841, a842) {
                            let a845 = {
                                let a843 = v0;
                                let a844 = 2u32;
                                fixed_u32::bit_and(a843, a844)
                            };
                            let a846 = 0u32;
                            if equal(a845, a846) {
                                let a849 = {
                                    let a847 = v0;
                                    let a848 = 1u32;
                                    fixed_u32::bit_and(a847, a848)
                                };
                                let a850 = 0u32;
                                if equal(a849, a850) {
                                    Bytes::lit(&[0x90])
                                } else {
                                    Bytes::lit(&[0x91])
                                }
                            } else {
                                let a853 = {
                                    let a851 = v0;
                                    let a852 = 1u32;
                                    fixed_u32::bit_and(a851, a852)
                                };
                                let a854 = 0u32;
                                if equal(a853, a854) {
                                    Bytes::lit(&[0x92])
                                } else {
                                    Bytes::lit(&[0x93])
                                }
                            }
                        } else {
                            let a857 = {
                                let a855 = v0;
                                let a856 = 2u32;
                                fixed_u32::bit_and(a855, a856)
                            };
                            let a858 = 0u32;
                            if equal(a857, a858) {
                                let a861 = {
                                    let a859 = v0;
                                    let a860 = 1u32;
                                    fixed_u32::bit_and(a859, a860)
                                };
                                let a862 = 0u32;
                                if equal(a861, a862) {
                                    Bytes::lit(&[0x94])
                                } else {
                                    Bytes::lit(&[0x95])
                                }
                            } else {
                                let a865 = {
                                    let a863 = v0;
                                    let a864 = 1u32;
                                    fixed_u32::bit_and(a863, a864)
                                };
                                let a866 = 0u32;
                                if equal(a865, a866) {
                                    Bytes::lit(&[0x96])
                                } else {
                                    Bytes::lit(&[0x97])
                                }
                            }
                        }
                    } else {
                        let a869 = {
                            let a867 = v0;
                            let a868 = 4u32;
                            fixed_u32::bit_and(a867, a868)
                        };
                        let a870 = 0u32;
                        if equal(a869, a870) {
                            let a873 = {
                                let a871 = v0;
                                let a872 = 2u32;
                                fixed_u32::bit_and(a871, a872)
                            };
                            let a874 = 0u32;
                            if equal(a873, a874) {
                                let a877 = {
                                    let a875 = v0;
                                    let a876 = 1u32;
                                    fixed_u32::bit_and(a875, a876)
                                };
                                let a878 = 0u32;
                                if equal(a877, a878) {
                                    Bytes::lit(&[0x98])
                                } else {
                                    Bytes::lit(&[0x99])
                                }
                            } else {
                                let a881 = {
                                    let a879 = v0;
                                    let a880 = 1u32;
                                    fixed_u32::bit_and(a879, a880)
                                };
                                let a882 = 0u32;
                                if equal(a881, a882) {
                                    Bytes::lit(&[0x9a])
                                } else {
                                    Bytes::lit(&[0x9b])
                                }
                            }
                        } else {
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
                                    Bytes::lit(&[0x9c])
                                } else {
                                    Bytes::lit(&[0x9d])
                                }
                            } else {
                                let a893 = {
                                    let a891 = v0;
                                    let a892 = 1u32;
                                    fixed_u32::bit_and(a891, a892)
                                };
                                let a894 = 0u32;
                                if equal(a893, a894) {
                                    Bytes::lit(&[0x9e])
                                } else {
                                    Bytes::lit(&[0x9f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a897 = {
                    let a895 = v0;
                    let a896 = 16u32;
                    fixed_u32::bit_and(a895, a896)
                };
                let a898 = 0u32;
                if equal(a897, a898) {
                    let a901 = {
                        let a899 = v0;
                        let a900 = 8u32;
                        fixed_u32::bit_and(a899, a900)
                    };
                    let a902 = 0u32;
                    if equal(a901, a902) {
                        let a905 = {
                            let a903 = v0;
                            let a904 = 4u32;
                            fixed_u32::bit_and(a903, a904)
                        };
                        let a906 = 0u32;
                        if equal(a905, a906) {
                            let a909 = {
                                let a907 = v0;
                                let a908 = 2u32;
                                fixed_u32::bit_and(a907, a908)
                            };
                            let a910 = 0u32;
                            if equal(a909, a910) {
                                let a913 = {
                                    let a911 = v0;
                                    let a912 = 1u32;
                                    fixed_u32::bit_and(a911, a912)
                                };
                                let a914 = 0u32;
                                if equal(a913, a914) {
                                    Bytes::lit(&[0xa0])
                                } else {
                                    Bytes::lit(&[0xa1])
                                }
                            } else {
                                let a917 = {
                                    let a915 = v0;
                                    let a916 = 1u32;
                                    fixed_u32::bit_and(a915, a916)
                                };
                                let a918 = 0u32;
                                if equal(a917, a918) {
                                    Bytes::lit(&[0xa2])
                                } else {
                                    Bytes::lit(&[0xa3])
                                }
                            }
                        } else {
                            let a921 = {
                                let a919 = v0;
                                let a920 = 2u32;
                                fixed_u32::bit_and(a919, a920)
                            };
                            let a922 = 0u32;
                            if equal(a921, a922) {
                                let a925 = {
                                    let a923 = v0;
                                    let a924 = 1u32;
                                    fixed_u32::bit_and(a923, a924)
                                };
                                let a926 = 0u32;
                                if equal(a925, a926) {
                                    Bytes::lit(&[0xa4])
                                } else {
                                    Bytes::lit(&[0xa5])
                                }
                            } else {
                                let a929 = {
                                    let a927 = v0;
                                    let a928 = 1u32;
                                    fixed_u32::bit_and(a927, a928)
                                };
                                let a930 = 0u32;
                                if equal(a929, a930) {
                                    Bytes::lit(&[0xa6])
                                } else {
                                    Bytes::lit(&[0xa7])
                                }
                            }
                        }
                    } else {
                        let a933 = {
                            let a931 = v0;
                            let a932 = 4u32;
                            fixed_u32::bit_and(a931, a932)
                        };
                        let a934 = 0u32;
                        if equal(a933, a934) {
                            let a937 = {
                                let a935 = v0;
                                let a936 = 2u32;
                                fixed_u32::bit_and(a935, a936)
                            };
                            let a938 = 0u32;
                            if equal(a937, a938) {
                                let a941 = {
                                    let a939 = v0;
                                    let a940 = 1u32;
                                    fixed_u32::bit_and(a939, a940)
                                };
                                let a942 = 0u32;
                                if equal(a941, a942) {
                                    Bytes::lit(&[0xa8])
                                } else {
                                    Bytes::lit(&[0xa9])
                                }
                            } else {
                                let a945 = {
                                    let a943 = v0;
                                    let a944 = 1u32;
                                    fixed_u32::bit_and(a943, a944)
                                };
                                let a946 = 0u32;
                                if equal(a945, a946) {
                                    Bytes::lit(&[0xaa])
                                } else {
                                    Bytes::lit(&[0xab])
                                }
                            }
                        } else {
                            let a949 = {
                                let a947 = v0;
                                let a948 = 2u32;
                                fixed_u32::bit_and(a947, a948)
                            };
                            let a950 = 0u32;
                            if equal(a949, a950) {
                                let a953 = {
                                    let a951 = v0;
                                    let a952 = 1u32;
                                    fixed_u32::bit_and(a951, a952)
                                };
                                let a954 = 0u32;
                                if equal(a953, a954) {
                                    Bytes::lit(&[0xac])
                                } else {
                                    Bytes::lit(&[0xad])
                                }
                            } else {
                                let a957 = {
                                    let a955 = v0;
                                    let a956 = 1u32;
                                    fixed_u32::bit_and(a955, a956)
                                };
                                let a958 = 0u32;
                                if equal(a957, a958) {
                                    Bytes::lit(&[0xae])
                                } else {
                                    Bytes::lit(&[0xaf])
                                }
                            }
                        }
                    }
                } else {
                    let a961 = {
                        let a959 = v0;
                        let a960 = 8u32;
                        fixed_u32::bit_and(a959, a960)
                    };
                    let a962 = 0u32;
                    if equal(a961, a962) {
                        let a965 = {
                            let a963 = v0;
                            let a964 = 4u32;
                            fixed_u32::bit_and(a963, a964)
                        };
                        let a966 = 0u32;
                        if equal(a965, a966) {
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
                                    Bytes::lit(&[0xb0])
                                } else {
                                    Bytes::lit(&[0xb1])
                                }
                            } else {
                                let a977 = {
                                    let a975 = v0;
                                    let a976 = 1u32;
                                    fixed_u32::bit_and(a975, a976)
                                };
                                let a978 = 0u32;
                                if equal(a977, a978) {
                                    Bytes::lit(&[0xb2])
                                } else {
                                    Bytes::lit(&[0xb3])
                                }
                            }
                        } else {
                            let a981 = {
                                let a979 = v0;
                                let a980 = 2u32;
                                fixed_u32::bit_and(a979, a980)
                            };
                            let a982 = 0u32;
                            if equal(a981, a982) {
                                let a985 = {
                                    let a983 = v0;
                                    let a984 = 1u32;
                                    fixed_u32::bit_and(a983, a984)
                                };
                                let a986 = 0u32;
                                if equal(a985, a986) {
                                    Bytes::lit(&[0xb4])
                                } else {
                                    Bytes::lit(&[0xb5])
                                }
                            } else {
                                let a989 = {
                                    let a987 = v0;
                                    let a988 = 1u32;
                                    fixed_u32::bit_and(a987, a988)
                                };
                                let a990 = 0u32;
                                if equal(a989, a990) {
                                    Bytes::lit(&[0xb6])
                                } else {
                                    Bytes::lit(&[0xb7])
                                }
                            }
                        }
                    } else {
                        let a993 = {
                            let a991 = v0;
                            let a992 = 4u32;
                            fixed_u32::bit_and(a991, a992)
                        };
                        let a994 = 0u32;
                        if equal(a993, a994) {
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
                                    Bytes::lit(&[0xb8])
                                } else {
                                    Bytes::lit(&[0xb9])
                                }
                            } else {
                                let a1005 = {
                                    let a1003 = v0;
                                    let a1004 = 1u32;
                                    fixed_u32::bit_and(a1003, a1004)
                                };
                                let a1006 = 0u32;
                                if equal(a1005, a1006) {
                                    Bytes::lit(&[0xba])
                                } else {
                                    Bytes::lit(&[0xbb])
                                }
                            }
                        } else {
                            let a1009 = {
                                let a1007 = v0;
                                let a1008 = 2u32;
                                fixed_u32::bit_and(a1007, a1008)
                            };
                            let a1010 = 0u32;
                            if equal(a1009, a1010) {
                                let a1013 = {
                                    let a1011 = v0;
                                    let a1012 = 1u32;
                                    fixed_u32::bit_and(a1011, a1012)
                                };
                                let a1014 = 0u32;
                                if equal(a1013, a1014) {
                                    Bytes::lit(&[0xbc])
                                } else {
                                    Bytes::lit(&[0xbd])
                                }
                            } else {
                                let a1017 = {
                                    let a1015 = v0;
                                    let a1016 = 1u32;
                                    fixed_u32::bit_and(a1015, a1016)
                                };
                                let a1018 = 0u32;
                                if equal(a1017, a1018) {
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
            let a1021 = {
                let a1019 = v0;
                let a1020 = 32u32;
                fixed_u32::bit_and(a1019, a1020)
            };
            let a1022 = 0u32;
            if equal(a1021, a1022) {
                let a1025 = {
                    let a1023 = v0;
                    let a1024 = 16u32;
                    fixed_u32::bit_and(a1023, a1024)
                };
                let a1026 = 0u32;
                if equal(a1025, a1026) {
                    let a1029 = {
                        let a1027 = v0;
                        let a1028 = 8u32;
                        fixed_u32::bit_and(a1027, a1028)
                    };
                    let a1030 = 0u32;
                    if equal(a1029, a1030) {
                        let a1033 = {
                            let a1031 = v0;
                            let a1032 = 4u32;
                            fixed_u32::bit_and(a1031, a1032)
                        };
                        let a1034 = 0u32;
                        if equal(a1033, a1034) {
                            let a1037 = {
                                let a1035 = v0;
                                let a1036 = 2u32;
                                fixed_u32::bit_and(a1035, a1036)
                            };
                            let a1038 = 0u32;
                            if equal(a1037, a1038) {
                                let a1041 = {
                                    let a1039 = v0;
                                    let a1040 = 1u32;
                                    fixed_u32::bit_and(a1039, a1040)
                                };
                                let a1042 = 0u32;
                                if equal(a1041, a1042) {
                                    Bytes::lit(&[0xc0])
                                } else {
                                    Bytes::lit(&[0xc1])
                                }
                            } else {
                                let a1045 = {
                                    let a1043 = v0;
                                    let a1044 = 1u32;
                                    fixed_u32::bit_and(a1043, a1044)
                                };
                                let a1046 = 0u32;
                                if equal(a1045, a1046) {
                                    Bytes::lit(&[0xc2])
                                } else {
                                    Bytes::lit(&[0xc3])
                                }
                            }
                        } else {
                            let a1049 = {
                                let a1047 = v0;
                                let a1048 = 2u32;
                                fixed_u32::bit_and(a1047, a1048)
                            };
                            let a1050 = 0u32;
                            if equal(a1049, a1050) {
                                let a1053 = {
                                    let a1051 = v0;
                                    let a1052 = 1u32;
                                    fixed_u32::bit_and(a1051, a1052)
                                };
                                let a1054 = 0u32;
                                if equal(a1053, a1054) {
                                    Bytes::lit(&[0xc4])
                                } else {
                                    Bytes::lit(&[0xc5])
                                }
                            } else {
                                let a1057 = {
                                    let a1055 = v0;
                                    let a1056 = 1u32;
                                    fixed_u32::bit_and(a1055, a1056)
                                };
                                let a1058 = 0u32;
                                if equal(a1057, a1058) {
                                    Bytes::lit(&[0xc6])
                                } else {
                                    Bytes::lit(&[0xc7])
                                }
                            }
                        }
                    } else {
                        let a1061 = {
                            let a1059 = v0;
                            let a1060 = 4u32;
                            fixed_u32::bit_and(a1059, a1060)
                        };
                        let a1062 = 0u32;
                        if equal(a1061, a1062) {
                            let a1065 = {
                                let a1063 = v0;
                                let a1064 = 2u32;
                                fixed_u32::bit_and(a1063, a1064)
                            };
                            let a1066 = 0u32;
                            if equal(a1065, a1066) {
                                let a1069 = {
                                    let a1067 = v0;
                                    let a1068 = 1u32;
                                    fixed_u32::bit_and(a1067, a1068)
                                };
                                let a1070 = 0u32;
                                if equal(a1069, a1070) {
                                    Bytes::lit(&[0xc8])
                                } else {
                                    Bytes::lit(&[0xc9])
                                }
                            } else {
                                let a1073 = {
                                    let a1071 = v0;
                                    let a1072 = 1u32;
                                    fixed_u32::bit_and(a1071, a1072)
                                };
                                let a1074 = 0u32;
                                if equal(a1073, a1074) {
                                    Bytes::lit(&[0xca])
                                } else {
                                    Bytes::lit(&[0xcb])
                                }
                            }
                        } else {
                            let a1077 = {
                                let a1075 = v0;
                                let a1076 = 2u32;
                                fixed_u32::bit_and(a1075, a1076)
                            };
                            let a1078 = 0u32;
                            if equal(a1077, a1078) {
                                let a1081 = {
                                    let a1079 = v0;
                                    let a1080 = 1u32;
                                    fixed_u32::bit_and(a1079, a1080)
                                };
                                let a1082 = 0u32;
                                if equal(a1081, a1082) {
                                    Bytes::lit(&[0xcc])
                                } else {
                                    Bytes::lit(&[0xcd])
                                }
                            } else {
                                let a1085 = {
                                    let a1083 = v0;
                                    let a1084 = 1u32;
                                    fixed_u32::bit_and(a1083, a1084)
                                };
                                let a1086 = 0u32;
                                if equal(a1085, a1086) {
                                    Bytes::lit(&[0xce])
                                } else {
                                    Bytes::lit(&[0xcf])
                                }
                            }
                        }
                    }
                } else {
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
                                    Bytes::lit(&[0xd0])
                                } else {
                                    Bytes::lit(&[0xd1])
                                }
                            } else {
                                let a1105 = {
                                    let a1103 = v0;
                                    let a1104 = 1u32;
                                    fixed_u32::bit_and(a1103, a1104)
                                };
                                let a1106 = 0u32;
                                if equal(a1105, a1106) {
                                    Bytes::lit(&[0xd2])
                                } else {
                                    Bytes::lit(&[0xd3])
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
                                    Bytes::lit(&[0xd4])
                                } else {
                                    Bytes::lit(&[0xd5])
                                }
                            } else {
                                let a1117 = {
                                    let a1115 = v0;
                                    let a1116 = 1u32;
                                    fixed_u32::bit_and(a1115, a1116)
                                };
                                let a1118 = 0u32;
                                if equal(a1117, a1118) {
                                    Bytes::lit(&[0xd6])
                                } else {
                                    Bytes::lit(&[0xd7])
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
                                    Bytes::lit(&[0xd8])
                                } else {
                                    Bytes::lit(&[0xd9])
                                }
                            } else {
                                let a1133 = {
                                    let a1131 = v0;
                                    let a1132 = 1u32;
                                    fixed_u32::bit_and(a1131, a1132)
                                };
                                let a1134 = 0u32;
                                if equal(a1133, a1134) {
                                    Bytes::lit(&[0xda])
                                } else {
                                    Bytes::lit(&[0xdb])
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
                                    Bytes::lit(&[0xdc])
                                } else {
                                    Bytes::lit(&[0xdd])
                                }
                            } else {
                                let a1145 = {
                                    let a1143 = v0;
                                    let a1144 = 1u32;
                                    fixed_u32::bit_and(a1143, a1144)
                                };
                                let a1146 = 0u32;
                                if equal(a1145, a1146) {
                                    Bytes::lit(&[0xde])
                                } else {
                                    Bytes::lit(&[0xdf])
                                }
                            }
                        }
                    }
                }
            } else {
                let a1149 = {
                    let a1147 = v0;
                    let a1148 = 16u32;
                    fixed_u32::bit_and(a1147, a1148)
                };
                let a1150 = 0u32;
                if equal(a1149, a1150) {
                    let a1153 = {
                        let a1151 = v0;
                        let a1152 = 8u32;
                        fixed_u32::bit_and(a1151, a1152)
                    };
                    let a1154 = 0u32;
                    if equal(a1153, a1154) {
                        let a1157 = {
                            let a1155 = v0;
                            let a1156 = 4u32;
                            fixed_u32::bit_and(a1155, a1156)
                        };
                        let a1158 = 0u32;
                        if equal(a1157, a1158) {
                            let a1161 = {
                                let a1159 = v0;
                                let a1160 = 2u32;
                                fixed_u32::bit_and(a1159, a1160)
                            };
                            let a1162 = 0u32;
                            if equal(a1161, a1162) {
                                let a1165 = {
                                    let a1163 = v0;
                                    let a1164 = 1u32;
                                    fixed_u32::bit_and(a1163, a1164)
                                };
                                let a1166 = 0u32;
                                if equal(a1165, a1166) {
                                    Bytes::lit(&[0xe0])
                                } else {
                                    Bytes::lit(&[0xe1])
                                }
                            } else {
                                let a1169 = {
                                    let a1167 = v0;
                                    let a1168 = 1u32;
                                    fixed_u32::bit_and(a1167, a1168)
                                };
                                let a1170 = 0u32;
                                if equal(a1169, a1170) {
                                    Bytes::lit(&[0xe2])
                                } else {
                                    Bytes::lit(&[0xe3])
                                }
                            }
                        } else {
                            let a1173 = {
                                let a1171 = v0;
                                let a1172 = 2u32;
                                fixed_u32::bit_and(a1171, a1172)
                            };
                            let a1174 = 0u32;
                            if equal(a1173, a1174) {
                                let a1177 = {
                                    let a1175 = v0;
                                    let a1176 = 1u32;
                                    fixed_u32::bit_and(a1175, a1176)
                                };
                                let a1178 = 0u32;
                                if equal(a1177, a1178) {
                                    Bytes::lit(&[0xe4])
                                } else {
                                    Bytes::lit(&[0xe5])
                                }
                            } else {
                                let a1181 = {
                                    let a1179 = v0;
                                    let a1180 = 1u32;
                                    fixed_u32::bit_and(a1179, a1180)
                                };
                                let a1182 = 0u32;
                                if equal(a1181, a1182) {
                                    Bytes::lit(&[0xe6])
                                } else {
                                    Bytes::lit(&[0xe7])
                                }
                            }
                        }
                    } else {
                        let a1185 = {
                            let a1183 = v0;
                            let a1184 = 4u32;
                            fixed_u32::bit_and(a1183, a1184)
                        };
                        let a1186 = 0u32;
                        if equal(a1185, a1186) {
                            let a1189 = {
                                let a1187 = v0;
                                let a1188 = 2u32;
                                fixed_u32::bit_and(a1187, a1188)
                            };
                            let a1190 = 0u32;
                            if equal(a1189, a1190) {
                                let a1193 = {
                                    let a1191 = v0;
                                    let a1192 = 1u32;
                                    fixed_u32::bit_and(a1191, a1192)
                                };
                                let a1194 = 0u32;
                                if equal(a1193, a1194) {
                                    Bytes::lit(&[0xe8])
                                } else {
                                    Bytes::lit(&[0xe9])
                                }
                            } else {
                                let a1197 = {
                                    let a1195 = v0;
                                    let a1196 = 1u32;
                                    fixed_u32::bit_and(a1195, a1196)
                                };
                                let a1198 = 0u32;
                                if equal(a1197, a1198) {
                                    Bytes::lit(&[0xea])
                                } else {
                                    Bytes::lit(&[0xeb])
                                }
                            }
                        } else {
                            let a1201 = {
                                let a1199 = v0;
                                let a1200 = 2u32;
                                fixed_u32::bit_and(a1199, a1200)
                            };
                            let a1202 = 0u32;
                            if equal(a1201, a1202) {
                                let a1205 = {
                                    let a1203 = v0;
                                    let a1204 = 1u32;
                                    fixed_u32::bit_and(a1203, a1204)
                                };
                                let a1206 = 0u32;
                                if equal(a1205, a1206) {
                                    Bytes::lit(&[0xec])
                                } else {
                                    Bytes::lit(&[0xed])
                                }
                            } else {
                                let a1209 = {
                                    let a1207 = v0;
                                    let a1208 = 1u32;
                                    fixed_u32::bit_and(a1207, a1208)
                                };
                                let a1210 = 0u32;
                                if equal(a1209, a1210) {
                                    Bytes::lit(&[0xee])
                                } else {
                                    Bytes::lit(&[0xef])
                                }
                            }
                        }
                    }
                } else {
                    let a1213 = {
                        let a1211 = v0;
                        let a1212 = 8u32;
                        fixed_u32::bit_and(a1211, a1212)
                    };
                    let a1214 = 0u32;
                    if equal(a1213, a1214) {
                        let a1217 = {
                            let a1215 = v0;
                            let a1216 = 4u32;
                            fixed_u32::bit_and(a1215, a1216)
                        };
                        let a1218 = 0u32;
                        if equal(a1217, a1218) {
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
                                    Bytes::lit(&[0xf0])
                                } else {
                                    Bytes::lit(&[0xf1])
                                }
                            } else {
                                let a1229 = {
                                    let a1227 = v0;
                                    let a1228 = 1u32;
                                    fixed_u32::bit_and(a1227, a1228)
                                };
                                let a1230 = 0u32;
                                if equal(a1229, a1230) {
                                    Bytes::lit(&[0xf2])
                                } else {
                                    Bytes::lit(&[0xf3])
                                }
                            }
                        } else {
                            let a1233 = {
                                let a1231 = v0;
                                let a1232 = 2u32;
                                fixed_u32::bit_and(a1231, a1232)
                            };
                            let a1234 = 0u32;
                            if equal(a1233, a1234) {
                                let a1237 = {
                                    let a1235 = v0;
                                    let a1236 = 1u32;
                                    fixed_u32::bit_and(a1235, a1236)
                                };
                                let a1238 = 0u32;
                                if equal(a1237, a1238) {
                                    Bytes::lit(&[0xf4])
                                } else {
                                    Bytes::lit(&[0xf5])
                                }
                            } else {
                                let a1241 = {
                                    let a1239 = v0;
                                    let a1240 = 1u32;
                                    fixed_u32::bit_and(a1239, a1240)
                                };
                                let a1242 = 0u32;
                                if equal(a1241, a1242) {
                                    Bytes::lit(&[0xf6])
                                } else {
                                    Bytes::lit(&[0xf7])
                                }
                            }
                        }
                    } else {
                        let a1245 = {
                            let a1243 = v0;
                            let a1244 = 4u32;
                            fixed_u32::bit_and(a1243, a1244)
                        };
                        let a1246 = 0u32;
                        if equal(a1245, a1246) {
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
                                    Bytes::lit(&[0xf8])
                                } else {
                                    Bytes::lit(&[0xf9])
                                }
                            } else {
                                let a1257 = {
                                    let a1255 = v0;
                                    let a1256 = 1u32;
                                    fixed_u32::bit_and(a1255, a1256)
                                };
                                let a1258 = 0u32;
                                if equal(a1257, a1258) {
                                    Bytes::lit(&[0xfa])
                                } else {
                                    Bytes::lit(&[0xfb])
                                }
                            }
                        } else {
                            let a1261 = {
                                let a1259 = v0;
                                let a1260 = 2u32;
                                fixed_u32::bit_and(a1259, a1260)
                            };
                            let a1262 = 0u32;
                            if equal(a1261, a1262) {
                                let a1265 = {
                                    let a1263 = v0;
                                    let a1264 = 1u32;
                                    fixed_u32::bit_and(a1263, a1264)
                                };
                                let a1266 = 0u32;
                                if equal(a1265, a1266) {
                                    Bytes::lit(&[0xfc])
                                } else {
                                    Bytes::lit(&[0xfd])
                                }
                            } else {
                                let a1269 = {
                                    let a1267 = v0;
                                    let a1268 = 1u32;
                                    fixed_u32::bit_and(a1267, a1268)
                                };
                                let a1270 = 0u32;
                                if equal(a1269, a1270) {
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

pub fn f41(v0: u32, v1: u32) -> u32 {
    let m1273 = {
        let a1271 = v0;
        let a1272 = v1;
        fixed_u32::shift_right(a1271, a1272)
    };
    match m1273 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f42(v0: u64, v1: u64) -> u32 {
    let (_, h1274) = {
        let (_, h1275) = f69(Fn13::F70, v1, (v0, (1u32, 0u32)));
        h1275
    };
    h1274
}

pub fn f43(v0: Adt4) -> Adt3 {
    Adt3::C0(match v0.clone() {
        Adt4::C0(h1276, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1276
        }
    }, match v0.clone() {
        Adt4::C0(_, h1277, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1277
        }
    }, match v0.clone() {
        Adt4::C0(_, _, h1278, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1278
        }
    }, match v0.clone() {
        Adt4::C0(_, _, _, h1279, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1279
        }
    }, match v0.clone() {
        Adt4::C0(_, _, _, _, h1280, _, _, _, _, _, _, _, _, _, _, _) => {
            h1280
        }
    }, match v0.clone() {
        Adt4::C0(_, _, _, _, _, h1281, _, _, _, _, _, _, _, _, _, _) => {
            h1281
        }
    }, match v0.clone() {
        Adt4::C0(_, _, _, _, _, _, h1282, _, _, _, _, _, _, _, _, _) => {
            h1282
        }
    }, match v0 {
        Adt4::C0(_, _, _, _, _, _, _, h1283, _, _, _, _, _, _, _, _) => {
            h1283
        }
    })
}

pub fn f44(v0: Bytes, v1: u64, v2: u64) -> R<Adt4> {
    Ok(Adt4::C0(f71(v0.clone(), v1, {
        let a1284 = v2;
        let a1285 = 0u64;
        nat_add(a1284, a1285)?
    })?, f71(v0.clone(), v1, {
        let a1286 = v2;
        let a1287 = 4u64;
        nat_add(a1286, a1287)?
    })?, f71(v0.clone(), v1, {
        let a1288 = v2;
        let a1289 = 8u64;
        nat_add(a1288, a1289)?
    })?, f71(v0.clone(), v1, {
        let a1290 = v2;
        let a1291 = 12u64;
        nat_add(a1290, a1291)?
    })?, f71(v0.clone(), v1, {
        let a1292 = v2;
        let a1293 = 16u64;
        nat_add(a1292, a1293)?
    })?, f71(v0.clone(), v1, {
        let a1294 = v2;
        let a1295 = 20u64;
        nat_add(a1294, a1295)?
    })?, f71(v0.clone(), v1, {
        let a1296 = v2;
        let a1297 = 24u64;
        nat_add(a1296, a1297)?
    })?, f71(v0.clone(), v1, {
        let a1298 = v2;
        let a1299 = 28u64;
        nat_add(a1298, a1299)?
    })?, f71(v0.clone(), v1, {
        let a1300 = v2;
        let a1301 = 32u64;
        nat_add(a1300, a1301)?
    })?, f71(v0.clone(), v1, {
        let a1302 = v2;
        let a1303 = 36u64;
        nat_add(a1302, a1303)?
    })?, f71(v0.clone(), v1, {
        let a1304 = v2;
        let a1305 = 40u64;
        nat_add(a1304, a1305)?
    })?, f71(v0.clone(), v1, {
        let a1306 = v2;
        let a1307 = 44u64;
        nat_add(a1306, a1307)?
    })?, f71(v0.clone(), v1, {
        let a1308 = v2;
        let a1309 = 48u64;
        nat_add(a1308, a1309)?
    })?, f71(v0.clone(), v1, {
        let a1310 = v2;
        let a1311 = 52u64;
        nat_add(a1310, a1311)?
    })?, f71(v0.clone(), v1, {
        let a1312 = v2;
        let a1313 = 56u64;
        nat_add(a1312, a1313)?
    })?, f71(v0, v1, {
        let a1314 = v2;
        let a1315 = 60u64;
        nat_add(a1314, a1315)?
    })?))
}

pub fn f45(v0: Bytes) -> R<Adt5> {
    f12(f35(f23(f24(), f57()), v0)?, 32u64)
}

pub fn f46(v0: Adt5, v1: u64) -> Adt4 {
    f72(Fn14::F73(v1), Adt4::C0(0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32), match v0 {
        Adt5::C0(_, _, _, _, _, _, h1316) => {
            h1316
        }
    })
}

pub fn f47() -> u32 {
    64u32
}

pub fn f48(v0: u64) -> R<u64> {
    let a1317 = v0;
    let a1318 = 0u64;
    if nat_eq(a1317, a1318) {
        Ok(0u64)
    } else {
        let a1324 = {
            let a1321 = {
                let a1319 = v0;
                let a1320 = 1u64;
                nat_sub(a1319, a1320)
            };
            let a1322 = 64u64;
            let a1323 = 0u64;
            nat_quot(a1321, a1322, a1323)
        };
        let a1325 = 64u64;
        nat_mul(a1324, a1325)
    }
}

pub fn f49(v0: Fn7, p1329: u64, p1330: (u64, (Adt3, List<Adt6>))) -> R<(u64, (Adt3, List<Adt6>))> {
    let m1328 = (p1329, p1330);
    {
        let mut m1328 = m1328;
        loop {
            let step = {
                let (v1, v2) = m1328;
                let m1326 = v1;
                if m1326 == 0 {
                    Err::<(u64, (u64, (Adt3, List<Adt6>))), R<(u64, (Adt3, List<Adt6>))>>(Ok(v2))
                } else {
                    let v3 = m1326 - 1;
                    Ok::<(u64, (u64, (Adt3, List<Adt6>))), R<(u64, (Adt3, List<Adt6>))>>((v3, {
                        let c1327: Fn7 = v0.clone();
                        c1327.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m1328 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f50(v0: u32, v1: u64, v2: u64, v3: Adt3, v4: u64, v5: Bytes, v6: (u64, (Adt3, List<Adt6>))) -> R<(u64, (Adt3, List<Adt6>))> {
    let v7: u64 = {
        let (h1331, _) = v6.clone();
        h1331
    };
    let v8: Adt2 = f54(f51(v3, v0, v5, v4, v1, v2, v7, {
        let (h1332, _) = {
            let (_, h1333) = v6.clone();
            h1333
        };
        h1332
    }, 64u64, f53(v7))?)?;
    Ok(({
        let a1334 = v7;
        let a1335 = 64u64;
        nat_add(a1334, a1335)?
    }, (f43(match v8.clone() {
        Adt2::C0(_, _, _, _, _, _, _, _, _, _, _, h1336) => {
            h1336
        }
    }), List::cons(Adt6::C0(v8), {
        let (_, h1337) = {
            let (_, h1338) = v6;
            h1338
        };
        h1337
    }))))
}

pub fn f51(_: Adt3, v1: u32, v2: Bytes, v3: u64, v4: u64, v5: u64, v6: u64, v7: Adt3, v8: u64, v9: u32) -> R<Adt9> {
    Ok(Adt9::C0(0u64, v5, 1u64, {
        let a1339 = v6;
        let a1340 = 64u64;
        let a1341 = 0u64;
        nat_quot(a1339, a1340, a1341)
    }, {
        let a1344 = {
            let a1342 = v5;
            let a1343 = 1024u64;
            nat_mul(a1342, a1343)?
        };
        let a1345 = v6;
        nat_add(a1344, a1345)?
    }, v7, f44(v2, v3, {
        let a1346 = v4;
        let a1347 = v6;
        nat_add(a1346, a1347)?
    })?, v5, f42(v8, 7u64), {
        let a1348 = v1;
        let a1349 = v9;
        fixed_u32::bit_or(a1348, a1349)
    }))
}

pub fn f52() -> u32 {
    2u32
}

pub fn f53(v0: u64) -> u32 {
    let a1350 = v0;
    let a1351 = 0u64;
    if nat_eq(a1350, a1351) {
        f74()
    } else {
        0u32
    }
}

pub fn f54(v0: Adt9) -> R<Adt2> {
    let v1: u32 = f21(match v0.clone() {
        Adt9::C0(_, _, _, _, _, _, _, h1352, _, _) => {
            h1352
        }
    });
    let v2: u32 = f22(match v0.clone() {
        Adt9::C0(_, _, _, _, _, _, _, h1353, _, _) => {
            h1353
        }
    });
    Ok(Adt2::C0(match v0.clone() {
        Adt9::C0(h1354, _, _, _, _, _, _, _, _, _) => {
            h1354
        }
    }, match v0.clone() {
        Adt9::C0(_, h1355, _, _, _, _, _, _, _, _) => {
            h1355
        }
    }, match v0.clone() {
        Adt9::C0(_, _, h1356, _, _, _, _, _, _, _) => {
            h1356
        }
    }, match v0.clone() {
        Adt9::C0(_, _, _, h1357, _, _, _, _, _, _) => {
            h1357
        }
    }, match v0.clone() {
        Adt9::C0(_, _, _, _, h1358, _, _, _, _, _) => {
            h1358
        }
    }, match v0.clone() {
        Adt9::C0(_, _, _, _, _, h1359, _, _, _, _) => {
            h1359
        }
    }, match v0.clone() {
        Adt9::C0(_, _, _, _, _, _, h1360, _, _, _) => {
            h1360
        }
    }, v1, v2, match v0.clone() {
        Adt9::C0(_, _, _, _, _, _, _, _, h1361, _) => {
            h1361
        }
    }, match v0.clone() {
        Adt9::C0(_, _, _, _, _, _, _, _, _, h1362) => {
            h1362
        }
    }, f75(match v0.clone() {
        Adt9::C0(_, _, _, _, _, h1363, _, _, _, _) => {
            h1363
        }
    }, match v0.clone() {
        Adt9::C0(_, _, _, _, _, _, h1364, _, _, _) => {
            h1364
        }
    }, v1, v2, match v0.clone() {
        Adt9::C0(_, _, _, _, _, _, _, _, h1365, _) => {
            h1365
        }
    }, match v0 {
        Adt9::C0(_, _, _, _, _, _, _, _, _, h1366) => {
            h1366
        }
    })?))
}

pub fn f55(v0: Adt7, v1: Adt7, v2: Adt3, v3: u32) -> R<Adt9> {
    Ok(Adt9::C0(1u64, match v0.clone() {
        Adt7::C0(_, h1367, _) => {
            h1367
        }
    }, {
        let a1370 = match v0.clone() {
            Adt7::C0(_, _, h1368) => {
                h1368
            }
        };
        let a1371 = match v1.clone() {
            Adt7::C0(_, _, h1369) => {
                h1369
            }
        };
        nat_add(a1370, a1371)?
    }, 0u64, 0u64, v2, Adt4::C0({
        let m1374 = match v0.clone() {
            Adt7::C0(h1373, _, _) => {
                h1373
            }
        };
        match m1374 {
            Adt3::C0(h1372, _, _, _, _, _, _, _) => {
                h1372
            }
        }
    }, {
        let m1377 = match v0.clone() {
            Adt7::C0(h1376, _, _) => {
                h1376
            }
        };
        match m1377 {
            Adt3::C0(_, h1375, _, _, _, _, _, _) => {
                h1375
            }
        }
    }, {
        let m1380 = match v0.clone() {
            Adt7::C0(h1379, _, _) => {
                h1379
            }
        };
        match m1380 {
            Adt3::C0(_, _, h1378, _, _, _, _, _) => {
                h1378
            }
        }
    }, {
        let m1383 = match v0.clone() {
            Adt7::C0(h1382, _, _) => {
                h1382
            }
        };
        match m1383 {
            Adt3::C0(_, _, _, h1381, _, _, _, _) => {
                h1381
            }
        }
    }, {
        let m1386 = match v0.clone() {
            Adt7::C0(h1385, _, _) => {
                h1385
            }
        };
        match m1386 {
            Adt3::C0(_, _, _, _, h1384, _, _, _) => {
                h1384
            }
        }
    }, {
        let m1389 = match v0.clone() {
            Adt7::C0(h1388, _, _) => {
                h1388
            }
        };
        match m1389 {
            Adt3::C0(_, _, _, _, _, h1387, _, _) => {
                h1387
            }
        }
    }, {
        let m1392 = match v0.clone() {
            Adt7::C0(h1391, _, _) => {
                h1391
            }
        };
        match m1392 {
            Adt3::C0(_, _, _, _, _, _, h1390, _) => {
                h1390
            }
        }
    }, {
        let m1395 = match v0 {
            Adt7::C0(h1394, _, _) => {
                h1394
            }
        };
        match m1395 {
            Adt3::C0(_, _, _, _, _, _, _, h1393) => {
                h1393
            }
        }
    }, {
        let m1398 = match v1.clone() {
            Adt7::C0(h1397, _, _) => {
                h1397
            }
        };
        match m1398 {
            Adt3::C0(h1396, _, _, _, _, _, _, _) => {
                h1396
            }
        }
    }, {
        let m1401 = match v1.clone() {
            Adt7::C0(h1400, _, _) => {
                h1400
            }
        };
        match m1401 {
            Adt3::C0(_, h1399, _, _, _, _, _, _) => {
                h1399
            }
        }
    }, {
        let m1404 = match v1.clone() {
            Adt7::C0(h1403, _, _) => {
                h1403
            }
        };
        match m1404 {
            Adt3::C0(_, _, h1402, _, _, _, _, _) => {
                h1402
            }
        }
    }, {
        let m1407 = match v1.clone() {
            Adt7::C0(h1406, _, _) => {
                h1406
            }
        };
        match m1407 {
            Adt3::C0(_, _, _, h1405, _, _, _, _) => {
                h1405
            }
        }
    }, {
        let m1410 = match v1.clone() {
            Adt7::C0(h1409, _, _) => {
                h1409
            }
        };
        match m1410 {
            Adt3::C0(_, _, _, _, h1408, _, _, _) => {
                h1408
            }
        }
    }, {
        let m1413 = match v1.clone() {
            Adt7::C0(h1412, _, _) => {
                h1412
            }
        };
        match m1413 {
            Adt3::C0(_, _, _, _, _, h1411, _, _) => {
                h1411
            }
        }
    }, {
        let m1416 = match v1.clone() {
            Adt7::C0(h1415, _, _) => {
                h1415
            }
        };
        match m1416 {
            Adt3::C0(_, _, _, _, _, _, h1414, _) => {
                h1414
            }
        }
    }, {
        let m1419 = match v1 {
            Adt7::C0(h1418, _, _) => {
                h1418
            }
        };
        match m1419 {
            Adt3::C0(_, _, _, _, _, _, _, h1417) => {
                h1417
            }
        }
    }), 0u64, 64u32, {
        let a1420 = v3;
        let a1421 = f76();
        fixed_u32::bit_or(a1420, a1421)
    }))
}

pub fn f56(v0: Adt9, v1: u64) -> Adt9 {
    Adt9::C0(2u64, match v0.clone() {
        Adt9::C0(_, h1422, _, _, _, _, _, _, _, _) => {
            h1422
        }
    }, match v0.clone() {
        Adt9::C0(_, _, h1423, _, _, _, _, _, _, _) => {
            h1423
        }
    }, v1, match v0.clone() {
        Adt9::C0(_, _, _, _, h1424, _, _, _, _, _) => {
            h1424
        }
    }, match v0.clone() {
        Adt9::C0(_, _, _, _, _, h1425, _, _, _, _) => {
            h1425
        }
    }, match v0.clone() {
        Adt9::C0(_, _, _, _, _, _, h1426, _, _, _) => {
            h1426
        }
    }, v1, match v0.clone() {
        Adt9::C0(_, _, _, _, _, _, _, _, h1427, _) => {
            h1427
        }
    }, {
        let a1429 = match v0 {
            Adt9::C0(_, _, _, _, _, _, _, _, _, h1428) => {
                h1428
            }
        };
        let a1430 = f77();
        fixed_u32::bit_or(a1429, a1430)
    })
}

pub fn f57() -> u32 {
    32u32
}

pub fn f58(v0: Fn8, p1434: List<Adt6>, p1435: List<Adt6>) -> List<Adt6> {
    let m1433 = (p1434, p1435);
    {
        let mut m1433 = m1433;
        loop {
            let step = {
                let (v1, v2) = m1433;
                let m1431 = v2;
                match m1431.uncons() {
                    None => {
                        Err::<(List<Adt6>, List<Adt6>), List<Adt6>>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(List<Adt6>, List<Adt6>), List<Adt6>>(({
                            let c1432: Fn8 = v0.clone();
                            c1432.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m1433 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f59(v0: List<Adt6>, v1: Adt6) -> List<Adt6> {
    List::cons(v1, v0)
}

pub fn f60(v0: Fn9, p1439: u64, p1440: (u64, (List<Adt7>, List<Adt6>))) -> R<(u64, (List<Adt7>, List<Adt6>))> {
    let m1438 = (p1439, p1440);
    {
        let mut m1438 = m1438;
        loop {
            let step = {
                let (v1, v2) = m1438;
                let m1436 = v1;
                if m1436 == 0 {
                    Err::<(u64, (u64, (List<Adt7>, List<Adt6>))), R<(u64, (List<Adt7>, List<Adt6>))>>(Ok(v2))
                } else {
                    let v3 = m1436 - 1;
                    Ok::<(u64, (u64, (List<Adt7>, List<Adt6>))), R<(u64, (List<Adt7>, List<Adt6>))>>((v3, {
                        let c1437: Fn9 = v0.clone();
                        c1437.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m1438 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f61(v0: u32, v1: Bytes, v2: Adt3, v3: Adt8, v4: (u64, (List<Adt7>, List<Adt6>))) -> R<(u64, (List<Adt7>, List<Adt6>))> {
    let v5: u64 = {
        let a1443 = match v3 {
            Adt8::C0(_, _, h1441, _, _, _, _) => {
                h1441
            }
        };
        let a1444 = {
            let (h1442, _) = v4.clone();
            h1442
        };
        nat_add(a1443, a1444)?
    };
    let v6: (Adt9, List<Adt6>) = f28(v2.clone(), v0, v1, {
        let a1446 = {
            let (h1445, _) = v4.clone();
            h1445
        };
        let a1447 = 1024u64;
        nat_mul(a1446, a1447)?
    }, 1024u64, v5, {
        let (_, h1448) = {
            let (_, h1449) = v4.clone();
            h1449
        };
        h1448
    })?;
    let v7: Adt2 = f54({
        let (h1450, _) = v6;
        h1450
    })?;
    Ok(({
        let a1452 = {
            let (h1451, _) = v4;
            h1451
        };
        let a1453 = 1u64;
        nat_add(a1452, a1453)?
    }, f78(v2, v0, Adt10::C0(Adt7::C0(f43(match v7.clone() {
        Adt2::C0(_, _, _, _, _, _, _, _, _, _, _, h1454) => {
            h1454
        }
    }), v5, 1u64), {
        let a1455 = v5;
        let a1456 = 1u64;
        nat_add(a1455, a1456)?
    }, {
        let (h1457, _) = {
            let (_, h1458) = v4;
            h1458
        };
        h1457
    }, List::cons(Adt6::C0(v7), {
        let (_, h1459) = v6;
        h1459
    })))?))
}

pub fn f62(v0: Fn10, p1464: u64, p1465: u64) -> R<(u64, bool)> {
    let m1463 = (p1464, p1465);
    {
        let mut m1463 = m1463;
        loop {
            let step = {
                let (v1, v2) = m1463;
                let m1460 = v1;
                if m1460 == 0 {
                    Err::<(u64, u64), R<(u64, bool)>>(Ok((v2, false)))
                } else {
                    let v3 = m1460 - 1;
                    let m1462 = {
                        let c1461: Fn10 = v0.clone();
                        c1461.apply(v2)?
                    };
                    match m1462 {
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
                Ok(next) => { m1463 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f63(v0: u64, v1: u64) -> R<Option<u64>> {
    let a1468 = {
        let a1466 = v1;
        let a1467 = 2u64;
        nat_mul(a1466, a1467)?
    };
    let a1469 = v0;
    if nat_lt(a1468, a1469) {
        Ok(Some({
            let a1470 = v1;
            let a1471 = 2u64;
            nat_mul(a1470, a1471)?
        }))
    } else {
        Ok(None::<u64>)
    }
}

pub fn f64(v0: Fn11, p1475: Adt4, p1476: List<Adt2>) -> Adt4 {
    let m1474 = (p1475, p1476);
    {
        let mut m1474 = m1474;
        loop {
            let step = {
                let (v1, v2) = m1474;
                let m1472 = v2;
                match m1472.uncons() {
                    None => {
                        Err::<(Adt4, List<Adt2>), Adt4>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt4, List<Adt2>), Adt4>(({
                            let c1473: Fn11 = v0.clone();
                            c1473.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m1474 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f65(v0: u64, v1: u64, v2: Adt4, v3: Adt2) -> Adt4 {
    if f79(v3.clone(), v0, v1) {
        match v3 {
            Adt2::C0(_, _, _, _, _, _, _, _, _, _, _, h1477) => {
                h1477
            }
        }
    } else {
        v2
    }
}

pub fn f66(v0: Fn12, p1481: (Option<Bytes>, List<Bytes>), p1482: List<Bytes>) -> (Option<Bytes>, List<Bytes>) {
    let m1480 = (p1481, p1482);
    {
        let mut m1480 = m1480;
        loop {
            let step = {
                let (v1, v2) = m1480;
                let m1478 = v2;
                match m1478.uncons() {
                    None => {
                        Err::<((Option<Bytes>, List<Bytes>), List<Bytes>), (Option<Bytes>, List<Bytes>)>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<((Option<Bytes>, List<Bytes>), List<Bytes>), (Option<Bytes>, List<Bytes>)>(({
                            let c1479: Fn12 = v0.clone();
                            c1479.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m1480 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f67(v0: (Option<Bytes>, List<Bytes>), v1: Bytes) -> (Option<Bytes>, List<Bytes>) {
    let m1484 = {
        let (h1483, _) = v0;
        h1483
    };
    match m1484 {
        None => {
            (Some(v1), {
                let (_, h1488) = v0;
                h1488
            })
        }
        Some(v2) => {
            (None::<Bytes>, List::cons({
                let a1485 = v2;
                let a1486 = v1;
                append_bytes(a1485, a1486)
            }, {
                let (_, h1487) = v0;
                h1487
            }))
        }
    }
}

pub fn f68(v0: List<Bytes>, v1: Bytes) -> List<Bytes> {
    List::cons(v1, v0)
}

pub fn f69(v0: Fn13, p1492: u64, p1493: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let m1491 = (p1492, p1493);
    {
        let mut m1491 = m1491;
        loop {
            let step = {
                let (v1, v2) = m1491;
                let m1489 = v1;
                if m1489 == 0 {
                    Err::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>(v2)
                } else {
                    let v3 = m1489 - 1;
                    Ok::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>((v3, {
                        let c1490: Fn13 = v0.clone();
                        c1490.apply(v2)
                    }))
                }
            };
            match step {
                Ok(next) => { m1491 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f70(v0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let v1: u64 = {
        let (h1494, _) = v0;
        h1494
    };
    let v2: u32 = {
        let (h1495, _) = {
            let (_, h1496) = v0;
            h1496
        };
        h1495
    };
    let v3: u32 = {
        let (_, h1497) = {
            let (_, h1498) = v0;
            h1498
        };
        h1497
    };
    ({
        let a1499 = v1;
        let a1500 = 2u64;
        let a1501 = 0u64;
        nat_quot(a1499, a1500, a1501)
    }, (f80(v2, 1u32), {
        let a1505 = {
            let a1502 = v1;
            let a1503 = 2u64;
            let a1504 = 0u64;
            nat_rem(a1502, a1503, a1504)
        };
        let a1506 = 1u64;
        if nat_eq(a1505, a1506) {
            let a1507 = v3;
            let a1508 = v2;
            fixed_u32::bit_or(a1507, a1508)
        } else {
            v3
        }
    }))
}

pub fn f71(v0: Bytes, v1: u64, v2: u64) -> R<u32> {
    Ok(f81(f82(v0.clone(), v1, v2), f82(v0.clone(), v1, {
        let a1509 = v2;
        let a1510 = 1u64;
        nat_add(a1509, a1510)?
    }), f82(v0.clone(), v1, {
        let a1511 = v2;
        let a1512 = 2u64;
        nat_add(a1511, a1512)?
    }), f82(v0, v1, {
        let a1513 = v2;
        let a1514 = 3u64;
        nat_add(a1513, a1514)?
    })))
}

pub fn f72(v0: Fn14, p1518: Adt4, p1519: List<Adt6>) -> Adt4 {
    let m1517 = (p1518, p1519);
    {
        let mut m1517 = m1517;
        loop {
            let step = {
                let (v1, v2) = m1517;
                let m1515 = v2;
                match m1515.uncons() {
                    None => {
                        Err::<(Adt4, List<Adt6>), Adt4>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt4, List<Adt6>), Adt4>(({
                            let c1516: Fn14 = v0.clone();
                            c1516.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m1517 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f73(v0: u64, v1: Adt4, v2: Adt6) -> Adt4 {
    let m1520 = v2;
    match m1520 {
        Adt6::C0(v3) => {
            let a1522 = match v3.clone() {
                Adt2::C0(h1521, _, _, _, _, _, _, _, _, _, _, _) => {
                    h1521
                }
            };
            let a1523 = 2u64;
            if nat_eq(a1522, a1523) {
                let a1525 = match v3.clone() {
                    Adt2::C0(_, _, _, h1524, _, _, _, _, _, _, _, _) => {
                        h1524
                    }
                };
                let a1526 = v0;
                if nat_eq(a1525, a1526) {
                    match v3 {
                        Adt2::C0(_, _, _, _, _, _, _, _, _, _, _, h1527) => {
                            h1527
                        }
                    }
                } else {
                    v1
                }
            } else {
                v1
            }
        }
        Adt6::C1(_, _) => {
            v1
        }
        Adt6::C2(_, _) => {
            v1
        }
    }
}

pub fn f74() -> u32 {
    1u32
}

pub fn f75(v0: Adt3, v1: Adt4, v2: u32, v3: u32, v4: u32, v5: u32) -> R<Adt4> {
    Ok(f83(f84(v0, v1, v2, v3, v4, v5)?))
}

pub fn f76() -> u32 {
    4u32
}

pub fn f77() -> u32 {
    8u32
}

pub fn f78(v0: Adt3, v1: u32, v2: Adt10) -> R<(List<Adt7>, List<Adt6>)> {
    let v3: Adt10 = {
        let (h1528, _) = f85(Fn15::F86(v1, v0), 64u64, v2)?;
        h1528
    };
    let v4: List<Adt7> = List::cons(match v3.clone() {
        Adt10::C0(h1529, _, _, _) => {
            h1529
        }
    }, match v3.clone() {
        Adt10::C0(_, _, h1530, _) => {
            h1530
        }
    });
    Ok((v4.clone(), List::cons(Adt6::C1(match v3.clone() {
        Adt10::C0(h1531, _, _, _) => {
            h1531
        }
    }, {
        let a1532 = v4;
        length_list(a1532)
    }), match v3 {
        Adt10::C0(_, _, _, h1533) => {
            h1533
        }
    })))
}

pub fn f79(v0: Adt2, v1: u64, v2: u64) -> bool {
    let a1534 = v2;
    let a1535 = 1u64;
    if nat_eq(a1534, a1535) {
        let a1537 = match v0.clone() {
            Adt2::C0(h1536, _, _, _, _, _, _, _, _, _, _, _) => {
                h1536
            }
        };
        let a1538 = 0u64;
        if nat_eq(a1537, a1538) {
            let a1540 = match v0.clone() {
                Adt2::C0(_, h1539, _, _, _, _, _, _, _, _, _, _) => {
                    h1539
                }
            };
            let a1541 = v1;
            if nat_eq(a1540, a1541) {
                let a1545 = {
                    let a1543 = match v0 {
                        Adt2::C0(_, _, _, _, _, _, _, _, _, _, h1542, _) => {
                            h1542
                        }
                    };
                    let a1544 = f52();
                    fixed_u32::bit_and(a1543, a1544)
                };
                let a1546 = 0u32;
                !equal(a1545, a1546)
            } else {
                false
            }
        } else {
            false
        }
    } else {
        let a1548 = match v0.clone() {
            Adt2::C0(h1547, _, _, _, _, _, _, _, _, _, _, _) => {
                h1547
            }
        };
        let a1549 = 1u64;
        if nat_eq(a1548, a1549) {
            let a1551 = match v0.clone() {
                Adt2::C0(_, h1550, _, _, _, _, _, _, _, _, _, _) => {
                    h1550
                }
            };
            let a1552 = v1;
            if nat_eq(a1551, a1552) {
                let a1554 = match v0 {
                    Adt2::C0(_, _, h1553, _, _, _, _, _, _, _, _, _) => {
                        h1553
                    }
                };
                let a1555 = v2;
                nat_eq(a1554, a1555)
            } else {
                false
            }
        } else {
            false
        }
    }
}

pub fn f80(v0: u32, v1: u32) -> u32 {
    let m1558 = {
        let a1556 = v0;
        let a1557 = v1;
        fixed_u32::shift_left(a1556, a1557)
    };
    match m1558 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f81(v0: u32, v1: u32, v2: u32, v3: u32) -> u32 {
    let a1563 = v0;
    let a1564 = {
        let a1561 = f80(v1, 8u32);
        let a1562 = {
            let a1559 = f80(v2, 16u32);
            let a1560 = f80(v3, 24u32);
            fixed_u32::bit_or(a1559, a1560)
        };
        fixed_u32::bit_or(a1561, a1562)
    };
    fixed_u32::bit_or(a1563, a1564)
}

pub fn f82(v0: Bytes, v1: u64, v2: u64) -> u32 {
    let a1565 = v2;
    let a1566 = v1;
    if nat_lt(a1565, a1566) {
        let m1569 = {
            let a1567 = v0;
            let a1568 = v2;
            index_bytes(a1567, a1568)
        };
        match m1569 {
            None => {
                0u32
            }
            Some(v3) => {
                let m1571 = {
                    let a1570 = v3;
                    fixed_u32::convert(i128::from(a1570))
                };
                match m1571 {
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

pub fn f83(v0: Adt11) -> Adt4 {
    f87(Fn16::F88, Adt4::C0(0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32), match v0 {
        Adt11::C0(_, _, _, _, _, _, h1572) => {
            h1572
        }
    })
}

pub fn f84(v0: Adt3, v1: Adt4, v2: u32, v3: u32, v4: u32, v5: u32) -> R<Adt11> {
    let v6: (u64, Adt14) = f89(Fn17::F90(v0.clone(), v1.clone()), 800u64, (0u64, Adt14::C0(f91(v0.clone(), v2, v3, v4, v5), Adt4::C0(0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32), List::<Adt12>::nil())))?;
    Ok(Adt11::C0(v0, v1, v2, v3, v4, v5, {
        let m1575 = {
            let (_, h1574) = v6;
            h1574
        };
        match m1575 {
            Adt14::C0(_, _, h1573) => {
                h1573
            }
        }
    }))
}

pub fn f85(v0: Fn15, p1580: u64, p1581: Adt10) -> R<(Adt10, bool)> {
    let m1579 = (p1580, p1581);
    {
        let mut m1579 = m1579;
        loop {
            let step = {
                let (v1, v2) = m1579;
                let m1576 = v1;
                if m1576 == 0 {
                    Err::<(u64, Adt10), R<(Adt10, bool)>>(Ok((v2, false)))
                } else {
                    let v3 = m1576 - 1;
                    let m1578 = {
                        let c1577: Fn15 = v0.clone();
                        c1577.apply(v2.clone())?
                    };
                    match m1578 {
                        None => {
                            Err::<(u64, Adt10), R<(Adt10, bool)>>(Ok((v2, true)))
                        }
                        Some(v4) => {
                            Ok::<(u64, Adt10), R<(Adt10, bool)>>((v3, v4))
                        }
                    }
                }
            };
            match step {
                Ok(next) => { m1579 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f86(v0: u32, v1: Adt3, v2: Adt10) -> R<Option<Adt10>> {
    f92(v1, v0, v2)
}

pub fn f87(v0: Fn16, p1585: Adt4, p1586: List<Adt12>) -> Adt4 {
    let m1584 = (p1585, p1586);
    {
        let mut m1584 = m1584;
        loop {
            let step = {
                let (v1, v2) = m1584;
                let m1582 = v2;
                match m1582.uncons() {
                    None => {
                        Err::<(Adt4, List<Adt12>), Adt4>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt4, List<Adt12>), Adt4>(({
                            let c1583: Fn16 = v0.clone();
                            c1583.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m1584 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f88(v0: Adt4, v1: Adt12) -> Adt4 {
    let a1588 = match v1.clone() {
        Adt12::C0(h1587, _, _, _, _, _, _, _, _) => {
            h1587
        }
    };
    let a1589 = 7u64;
    if nat_eq(a1588, a1589) {
        f93(v0, match v1.clone() {
            Adt12::C0(_, _, _, _, h1590, _, _, _, _) => {
                h1590
            }
        }, match v1 {
            Adt12::C0(_, _, _, _, _, _, _, _, h1591) => {
                h1591
            }
        })
    } else {
        v0
    }
}

pub fn f89(v0: Fn17, p1595: u64, p1596: (u64, Adt14)) -> R<(u64, Adt14)> {
    let m1594 = (p1595, p1596);
    {
        let mut m1594 = m1594;
        loop {
            let step = {
                let (v1, v2) = m1594;
                let m1592 = v1;
                if m1592 == 0 {
                    Err::<(u64, (u64, Adt14)), R<(u64, Adt14)>>(Ok(v2))
                } else {
                    let v3 = m1592 - 1;
                    Ok::<(u64, (u64, Adt14)), R<(u64, Adt14)>>((v3, {
                        let c1593: Fn17 = v0.clone();
                        c1593.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m1594 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f90(v0: Adt3, v1: Adt4, v2: (u64, Adt14)) -> R<(u64, Adt14)> {
    Ok(({
        let a1598 = {
            let (h1597, _) = v2.clone();
            h1597
        };
        let a1599 = 1u64;
        nat_add(a1598, a1599)?
    }, f94(v0, v1, {
        let (_, h1600) = v2;
        h1600
    }, {
        let (h1601, _) = v2;
        h1601
    })?))
}

pub fn f91(v0: Adt3, v1: u32, v2: u32, v3: u32, v4: u32) -> Adt4 {
    Adt4::C0(match v0.clone() {
        Adt3::C0(h1602, _, _, _, _, _, _, _) => {
            h1602
        }
    }, match v0.clone() {
        Adt3::C0(_, h1603, _, _, _, _, _, _) => {
            h1603
        }
    }, match v0.clone() {
        Adt3::C0(_, _, h1604, _, _, _, _, _) => {
            h1604
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h1605, _, _, _, _) => {
            h1605
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h1606, _, _, _) => {
            h1606
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h1607, _, _) => {
            h1607
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h1608, _) => {
            h1608
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, h1609) => {
            h1609
        }
    }, 1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, v1, v2, v3, v4)
}

pub fn f92(v0: Adt3, v1: u32, v2: Adt10) -> R<Option<Adt10>> {
    let a1614 = {
        let a1611 = match v2.clone() {
            Adt10::C0(_, h1610, _, _) => {
                h1610
            }
        };
        let a1612 = 2u64;
        let a1613 = 0u64;
        nat_rem(a1611, a1612, a1613)
    };
    let a1615 = 0u64;
    if nat_eq(a1614, a1615) {
        let m1617 = match v2.clone() {
            Adt10::C0(_, _, h1616, _) => {
                h1616
            }
        };
        match m1617.uncons() {
            None => {
                Ok(None::<Adt10>)
            }
            Some((v3, v4)) => {
                let v5: Adt2 = f54(f55(v3.clone(), match v2.clone() {
                    Adt10::C0(h1618, _, _, _) => {
                        h1618
                    }
                }, v0, v1)?)?;
                Ok(Some(Adt10::C0(Adt7::C0(f43(match v5.clone() {
                    Adt2::C0(_, _, _, _, _, _, _, _, _, _, _, h1619) => {
                        h1619
                    }
                }), match v3.clone() {
                    Adt7::C0(_, h1620, _) => {
                        h1620
                    }
                }, {
                    let a1625 = match v3.clone() {
                        Adt7::C0(_, _, h1621) => {
                            h1621
                        }
                    };
                    let a1626 = {
                        let m1624 = match v2.clone() {
                            Adt10::C0(h1623, _, _, _) => {
                                h1623
                            }
                        };
                        match m1624 {
                            Adt7::C0(_, _, h1622) => {
                                h1622
                            }
                        }
                    };
                    nat_add(a1625, a1626)?
                }), {
                    let a1628 = match v2.clone() {
                        Adt10::C0(_, h1627, _, _) => {
                            h1627
                        }
                    };
                    let a1629 = 2u64;
                    let a1630 = 0u64;
                    nat_quot(a1628, a1629, a1630)
                }, v4, List::cons(Adt6::C0(v5), List::cons(Adt6::C2(v3, {
                    let a1632 = match v2.clone() {
                        Adt10::C0(_, _, h1631, _) => {
                            h1631
                        }
                    };
                    length_list(a1632)
                }), match v2 {
                    Adt10::C0(_, _, _, h1633) => {
                        h1633
                    }
                })))))
            }
        }
    } else {
        Ok(None::<Adt10>)
    }
}

pub fn f93(v0: Adt4, v1: u64, v2: u32) -> Adt4 {
    let a1634 = v1;
    let a1635 = 8u64;
    if nat_lt(a1634, a1635) {
        let a1636 = v1;
        let a1637 = 4u64;
        if nat_lt(a1636, a1637) {
            let a1638 = v1;
            let a1639 = 2u64;
            if nat_lt(a1638, a1639) {
                let a1640 = v1;
                let a1641 = 1u64;
                if nat_lt(a1640, a1641) {
                    Adt4::C0(v2, match v0.clone() {
                        Adt4::C0(_, h1642, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1642
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1643, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1643
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1644, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1644
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1645, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1645
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1646, _, _, _, _, _, _, _, _, _, _) => {
                            h1646
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1647, _, _, _, _, _, _, _, _, _) => {
                            h1647
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1648, _, _, _, _, _, _, _, _) => {
                            h1648
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1649, _, _, _, _, _, _, _) => {
                            h1649
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1650, _, _, _, _, _, _) => {
                            h1650
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1651, _, _, _, _, _) => {
                            h1651
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1652, _, _, _, _) => {
                            h1652
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1653, _, _, _) => {
                            h1653
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1654, _, _) => {
                            h1654
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1655, _) => {
                            h1655
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1656) => {
                            h1656
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1657, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1657
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, h1658, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1658
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1659, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1659
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1660, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1660
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1661, _, _, _, _, _, _, _, _, _, _) => {
                            h1661
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1662, _, _, _, _, _, _, _, _, _) => {
                            h1662
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1663, _, _, _, _, _, _, _, _) => {
                            h1663
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1664, _, _, _, _, _, _, _) => {
                            h1664
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1665, _, _, _, _, _, _) => {
                            h1665
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1666, _, _, _, _, _) => {
                            h1666
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1667, _, _, _, _) => {
                            h1667
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1668, _, _, _) => {
                            h1668
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1669, _, _) => {
                            h1669
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1670, _) => {
                            h1670
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1671) => {
                            h1671
                        }
                    })
                }
            } else {
                let a1672 = v1;
                let a1673 = 3u64;
                if nat_lt(a1672, a1673) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1674, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1674
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1675, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1675
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, h1676, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1676
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1677, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1677
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1678, _, _, _, _, _, _, _, _, _, _) => {
                            h1678
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1679, _, _, _, _, _, _, _, _, _) => {
                            h1679
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1680, _, _, _, _, _, _, _, _) => {
                            h1680
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1681, _, _, _, _, _, _, _) => {
                            h1681
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1682, _, _, _, _, _, _) => {
                            h1682
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1683, _, _, _, _, _) => {
                            h1683
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1684, _, _, _, _) => {
                            h1684
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1685, _, _, _) => {
                            h1685
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1686, _, _) => {
                            h1686
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1687, _) => {
                            h1687
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1688) => {
                            h1688
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1689, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1689
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1690, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1690
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1691, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1691
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1692, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1692
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1693, _, _, _, _, _, _, _, _, _, _) => {
                            h1693
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1694, _, _, _, _, _, _, _, _, _) => {
                            h1694
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1695, _, _, _, _, _, _, _, _) => {
                            h1695
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1696, _, _, _, _, _, _, _) => {
                            h1696
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1697, _, _, _, _, _, _) => {
                            h1697
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1698, _, _, _, _, _) => {
                            h1698
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1699, _, _, _, _) => {
                            h1699
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1700, _, _, _) => {
                            h1700
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1701, _, _) => {
                            h1701
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1702, _) => {
                            h1702
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1703) => {
                            h1703
                        }
                    })
                }
            }
        } else {
            let a1704 = v1;
            let a1705 = 6u64;
            if nat_lt(a1704, a1705) {
                let a1706 = v1;
                let a1707 = 5u64;
                if nat_lt(a1706, a1707) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1708, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1708
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1709, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1709
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1710, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1710
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1711, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1711
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1712, _, _, _, _, _, _, _, _, _, _) => {
                            h1712
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1713, _, _, _, _, _, _, _, _, _) => {
                            h1713
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1714, _, _, _, _, _, _, _, _) => {
                            h1714
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1715, _, _, _, _, _, _, _) => {
                            h1715
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1716, _, _, _, _, _, _) => {
                            h1716
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1717, _, _, _, _, _) => {
                            h1717
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1718, _, _, _, _) => {
                            h1718
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1719, _, _, _) => {
                            h1719
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1720, _, _) => {
                            h1720
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1721, _) => {
                            h1721
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1722) => {
                            h1722
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1723, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1723
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1724, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1724
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1725, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1725
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1726, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1726
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1727, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1727
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1728, _, _, _, _, _, _, _, _, _) => {
                            h1728
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1729, _, _, _, _, _, _, _, _) => {
                            h1729
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1730, _, _, _, _, _, _, _) => {
                            h1730
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1731, _, _, _, _, _, _) => {
                            h1731
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1732, _, _, _, _, _) => {
                            h1732
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1733, _, _, _, _) => {
                            h1733
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1734, _, _, _) => {
                            h1734
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1735, _, _) => {
                            h1735
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1736, _) => {
                            h1736
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1737) => {
                            h1737
                        }
                    })
                }
            } else {
                let a1738 = v1;
                let a1739 = 7u64;
                if nat_lt(a1738, a1739) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1740, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1740
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1741, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1741
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1742, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1742
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1743, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1743
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1744, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1744
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1745, _, _, _, _, _, _, _, _, _, _) => {
                            h1745
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1746, _, _, _, _, _, _, _, _) => {
                            h1746
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1747, _, _, _, _, _, _, _) => {
                            h1747
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1748, _, _, _, _, _, _) => {
                            h1748
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1749, _, _, _, _, _) => {
                            h1749
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1750, _, _, _, _) => {
                            h1750
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1751, _, _, _) => {
                            h1751
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1752, _, _) => {
                            h1752
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1753, _) => {
                            h1753
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1754) => {
                            h1754
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1755, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1755
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1756, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1756
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1757, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1757
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1758, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1758
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1759, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1759
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1760, _, _, _, _, _, _, _, _, _, _) => {
                            h1760
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1761, _, _, _, _, _, _, _, _, _) => {
                            h1761
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1762, _, _, _, _, _, _, _) => {
                            h1762
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1763, _, _, _, _, _, _) => {
                            h1763
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1764, _, _, _, _, _) => {
                            h1764
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1765, _, _, _, _) => {
                            h1765
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1766, _, _, _) => {
                            h1766
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1767, _, _) => {
                            h1767
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1768, _) => {
                            h1768
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1769) => {
                            h1769
                        }
                    })
                }
            }
        }
    } else {
        let a1770 = v1;
        let a1771 = 12u64;
        if nat_lt(a1770, a1771) {
            let a1772 = v1;
            let a1773 = 10u64;
            if nat_lt(a1772, a1773) {
                let a1774 = v1;
                let a1775 = 9u64;
                if nat_lt(a1774, a1775) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1776, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1776
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1777, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1777
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1778, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1778
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1779, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1779
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1780, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1780
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1781, _, _, _, _, _, _, _, _, _, _) => {
                            h1781
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1782, _, _, _, _, _, _, _, _, _) => {
                            h1782
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1783, _, _, _, _, _, _, _, _) => {
                            h1783
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1784, _, _, _, _, _, _) => {
                            h1784
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1785, _, _, _, _, _) => {
                            h1785
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1786, _, _, _, _) => {
                            h1786
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1787, _, _, _) => {
                            h1787
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1788, _, _) => {
                            h1788
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1789, _) => {
                            h1789
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1790) => {
                            h1790
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1791, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1791
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1792, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1792
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1793, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1793
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1794, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1794
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1795, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1795
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1796, _, _, _, _, _, _, _, _, _, _) => {
                            h1796
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1797, _, _, _, _, _, _, _, _, _) => {
                            h1797
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1798, _, _, _, _, _, _, _, _) => {
                            h1798
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1799, _, _, _, _, _, _, _) => {
                            h1799
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1800, _, _, _, _, _) => {
                            h1800
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1801, _, _, _, _) => {
                            h1801
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1802, _, _, _) => {
                            h1802
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1803, _, _) => {
                            h1803
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1804, _) => {
                            h1804
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1805) => {
                            h1805
                        }
                    })
                }
            } else {
                let a1806 = v1;
                let a1807 = 11u64;
                if nat_lt(a1806, a1807) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1808, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1808
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1809, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1809
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1810, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1810
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1811, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1811
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1812, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1812
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1813, _, _, _, _, _, _, _, _, _, _) => {
                            h1813
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1814, _, _, _, _, _, _, _, _, _) => {
                            h1814
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1815, _, _, _, _, _, _, _, _) => {
                            h1815
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1816, _, _, _, _, _, _, _) => {
                            h1816
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1817, _, _, _, _, _, _) => {
                            h1817
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1818, _, _, _, _) => {
                            h1818
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1819, _, _, _) => {
                            h1819
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1820, _, _) => {
                            h1820
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1821, _) => {
                            h1821
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1822) => {
                            h1822
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1823, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1823
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1824, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1824
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1825, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1825
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1826, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1826
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1827, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1827
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1828, _, _, _, _, _, _, _, _, _, _) => {
                            h1828
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1829, _, _, _, _, _, _, _, _, _) => {
                            h1829
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1830, _, _, _, _, _, _, _, _) => {
                            h1830
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1831, _, _, _, _, _, _, _) => {
                            h1831
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1832, _, _, _, _, _, _) => {
                            h1832
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1833, _, _, _, _, _) => {
                            h1833
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1834, _, _, _) => {
                            h1834
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1835, _, _) => {
                            h1835
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1836, _) => {
                            h1836
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1837) => {
                            h1837
                        }
                    })
                }
            }
        } else {
            let a1838 = v1;
            let a1839 = 14u64;
            if nat_lt(a1838, a1839) {
                let a1840 = v1;
                let a1841 = 13u64;
                if nat_lt(a1840, a1841) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1842, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1842
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1843, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1843
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1844, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1844
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1845, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1845
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1846, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1846
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1847, _, _, _, _, _, _, _, _, _, _) => {
                            h1847
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1848, _, _, _, _, _, _, _, _, _) => {
                            h1848
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1849, _, _, _, _, _, _, _, _) => {
                            h1849
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1850, _, _, _, _, _, _, _) => {
                            h1850
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1851, _, _, _, _, _, _) => {
                            h1851
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1852, _, _, _, _, _) => {
                            h1852
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1853, _, _, _, _) => {
                            h1853
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1854, _, _) => {
                            h1854
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1855, _) => {
                            h1855
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1856) => {
                            h1856
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1857, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1857
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1858, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1858
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1859, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1859
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1860, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1860
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1861, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1861
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1862, _, _, _, _, _, _, _, _, _, _) => {
                            h1862
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1863, _, _, _, _, _, _, _, _, _) => {
                            h1863
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1864, _, _, _, _, _, _, _, _) => {
                            h1864
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1865, _, _, _, _, _, _, _) => {
                            h1865
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1866, _, _, _, _, _, _) => {
                            h1866
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1867, _, _, _, _, _) => {
                            h1867
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1868, _, _, _, _) => {
                            h1868
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1869, _, _, _) => {
                            h1869
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1870, _) => {
                            h1870
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1871) => {
                            h1871
                        }
                    })
                }
            } else {
                let a1872 = v1;
                let a1873 = 15u64;
                if nat_lt(a1872, a1873) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1874, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1874
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1875, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1875
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1876, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1876
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1877, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1877
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1878, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1878
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1879, _, _, _, _, _, _, _, _, _, _) => {
                            h1879
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1880, _, _, _, _, _, _, _, _, _) => {
                            h1880
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1881, _, _, _, _, _, _, _, _) => {
                            h1881
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1882, _, _, _, _, _, _, _) => {
                            h1882
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1883, _, _, _, _, _, _) => {
                            h1883
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1884, _, _, _, _, _) => {
                            h1884
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1885, _, _, _, _) => {
                            h1885
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1886, _, _, _) => {
                            h1886
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1887, _, _) => {
                            h1887
                        }
                    }, v2, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1888) => {
                            h1888
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1889, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1889
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1890, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1890
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1891, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1891
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1892, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1892
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1893, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1893
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1894, _, _, _, _, _, _, _, _, _, _) => {
                            h1894
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1895, _, _, _, _, _, _, _, _, _) => {
                            h1895
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1896, _, _, _, _, _, _, _, _) => {
                            h1896
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1897, _, _, _, _, _, _, _) => {
                            h1897
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1898, _, _, _, _, _, _) => {
                            h1898
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1899, _, _, _, _, _) => {
                            h1899
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1900, _, _, _, _) => {
                            h1900
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1901, _, _, _) => {
                            h1901
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1902, _, _) => {
                            h1902
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1903, _) => {
                            h1903
                        }
                    }, v2)
                }
            }
        }
    }
}

pub fn f94(v0: Adt3, v1: Adt4, v2: Adt14, v3: u64) -> R<Adt14> {
    let v4: u64 = {
        let a1904 = v3;
        let a1905 = 784u64;
        if nat_lt(a1904, a1905) {
            let a1906 = v3;
            let a1907 = 112u64;
            let a1908 = 0u64;
            nat_quot(a1906, a1907, a1908)
        } else {
            7u64
        }
    };
    let v5: Adt15 = f95({
        let a1909 = v3;
        let a1910 = 784u64;
        if nat_lt(a1909, a1910) {
            let a1911 = v3;
            let a1912 = 112u64;
            let a1913 = 0u64;
            nat_rem(a1911, a1912, a1913)
        } else {
            let a1914 = v3;
            let a1915 = 672u64;
            nat_sub(a1914, a1915)
        }
    });
    let v6: Adt4 = match v2.clone() {
        Adt14::C0(h1916, _, _) => {
            h1916
        }
    };
    let v7: u32 = f96(v6.clone(), match v5.clone() {
        Adt15::C0(_, _, _, h1917, _) => {
            h1917
        }
    });
    let v8: u32 = {
        let m1919 = match v5.clone() {
            Adt15::C0(_, _, h1918, _, _) => {
                h1918
            }
        };
        match m1919 {
            Adt13::C0 => {
                f96(v6.clone(), match v5.clone() {
                    Adt15::C0(_, _, _, _, h1920) => {
                        h1920
                    }
                })
            }
            Adt13::C1 => {
                f96(v1, f97(v4, match v5.clone() {
                    Adt15::C0(_, _, _, _, h1921) => {
                        h1921
                    }
                })?)
            }
            Adt13::C2 => {
                f96(v6.clone(), match v5.clone() {
                    Adt15::C0(_, _, _, _, h1922) => {
                        h1922
                    }
                })
            }
            Adt13::C3 => {
                f98(match v5.clone() {
                    Adt15::C0(_, _, _, _, h1923) => {
                        h1923
                    }
                })
            }
            Adt13::C4 => {
                f99(v0, match v5.clone() {
                    Adt15::C0(_, _, _, _, h1924) => {
                        h1924
                    }
                })
            }
        }
    };
    let v9: u32 = f100(match v5.clone() {
        Adt15::C0(_, _, h1925, _, _) => {
            h1925
        }
    }, v7, v8);
    Ok(Adt14::C0({
        let a1926 = v4;
        let a1927 = 7u64;
        if nat_eq(a1926, a1927) {
            v6
        } else {
            f93(v6, match v5.clone() {
                Adt15::C0(_, _, _, h1928, _) => {
                    h1928
                }
            }, v9)
        }
    }, {
        let a1929 = v4;
        let a1930 = 7u64;
        if nat_eq(a1929, a1930) {
            f93(match v2.clone() {
                Adt14::C0(_, h1931, _) => {
                    h1931
                }
            }, match v5.clone() {
                Adt15::C0(_, _, _, h1932, _) => {
                    h1932
                }
            }, v9)
        } else {
            match v2.clone() {
                Adt14::C0(_, h1933, _) => {
                    h1933
                }
            }
        }
    }, List::cons(Adt12::C0(v4, match v5.clone() {
        Adt15::C0(h1934, _, _, _, _) => {
            h1934
        }
    }, match v5.clone() {
        Adt15::C0(_, h1935, _, _, _) => {
            h1935
        }
    }, match v5.clone() {
        Adt15::C0(_, _, h1936, _, _) => {
            h1936
        }
    }, match v5.clone() {
        Adt15::C0(_, _, _, h1937, _) => {
            h1937
        }
    }, match v5 {
        Adt15::C0(_, _, _, _, h1938) => {
            h1938
        }
    }, v7, v8, v9), match v2 {
        Adt14::C0(_, _, h1939) => {
            h1939
        }
    })))
}

pub fn f95(v0: u64) -> Adt15 {
    let a1940 = v0;
    let a1941 = 64u64;
    if nat_lt(a1940, a1941) {
        let a1942 = v0;
        let a1943 = 32u64;
        if nat_lt(a1942, a1943) {
            let a1944 = v0;
            let a1945 = 16u64;
            if nat_lt(a1944, a1945) {
                let a1946 = v0;
                let a1947 = 8u64;
                if nat_lt(a1946, a1947) {
                    let a1948 = v0;
                    let a1949 = 4u64;
                    if nat_lt(a1948, a1949) {
                        let a1950 = v0;
                        let a1951 = 2u64;
                        if nat_lt(a1950, a1951) {
                            let a1952 = v0;
                            let a1953 = 1u64;
                            if nat_lt(a1952, a1953) {
                                Adt15::C0(0u64, 0u64, Adt13::C0, 0u64, 4u64)
                            } else {
                                Adt15::C0(0u64, 1u64, Adt13::C1, 0u64, 0u64)
                            }
                        } else {
                            let a1954 = v0;
                            let a1955 = 3u64;
                            if nat_lt(a1954, a1955) {
                                Adt15::C0(0u64, 2u64, Adt13::C2, 12u64, 0u64)
                            } else {
                                Adt15::C0(0u64, 3u64, Adt13::C3, 12u64, 16u64)
                            }
                        }
                    } else {
                        let a1956 = v0;
                        let a1957 = 6u64;
                        if nat_lt(a1956, a1957) {
                            let a1958 = v0;
                            let a1959 = 5u64;
                            if nat_lt(a1958, a1959) {
                                Adt15::C0(0u64, 4u64, Adt13::C0, 8u64, 12u64)
                            } else {
                                Adt15::C0(0u64, 5u64, Adt13::C2, 4u64, 8u64)
                            }
                        } else {
                            let a1960 = v0;
                            let a1961 = 7u64;
                            if nat_lt(a1960, a1961) {
                                Adt15::C0(0u64, 6u64, Adt13::C3, 4u64, 12u64)
                            } else {
                                Adt15::C0(0u64, 7u64, Adt13::C0, 0u64, 4u64)
                            }
                        }
                    }
                } else {
                    let a1962 = v0;
                    let a1963 = 12u64;
                    if nat_lt(a1962, a1963) {
                        let a1964 = v0;
                        let a1965 = 10u64;
                        if nat_lt(a1964, a1965) {
                            let a1966 = v0;
                            let a1967 = 9u64;
                            if nat_lt(a1966, a1967) {
                                Adt15::C0(0u64, 8u64, Adt13::C1, 0u64, 1u64)
                            } else {
                                Adt15::C0(0u64, 9u64, Adt13::C2, 12u64, 0u64)
                            }
                        } else {
                            let a1968 = v0;
                            let a1969 = 11u64;
                            if nat_lt(a1968, a1969) {
                                Adt15::C0(0u64, 10u64, Adt13::C3, 12u64, 8u64)
                            } else {
                                Adt15::C0(0u64, 11u64, Adt13::C0, 8u64, 12u64)
                            }
                        }
                    } else {
                        let a1970 = v0;
                        let a1971 = 14u64;
                        if nat_lt(a1970, a1971) {
                            let a1972 = v0;
                            let a1973 = 13u64;
                            if nat_lt(a1972, a1973) {
                                Adt15::C0(0u64, 12u64, Adt13::C2, 4u64, 8u64)
                            } else {
                                Adt15::C0(0u64, 13u64, Adt13::C3, 4u64, 7u64)
                            }
                        } else {
                            let a1974 = v0;
                            let a1975 = 15u64;
                            if nat_lt(a1974, a1975) {
                                Adt15::C0(1u64, 0u64, Adt13::C0, 1u64, 5u64)
                            } else {
                                Adt15::C0(1u64, 1u64, Adt13::C1, 1u64, 2u64)
                            }
                        }
                    }
                }
            } else {
                let a1976 = v0;
                let a1977 = 24u64;
                if nat_lt(a1976, a1977) {
                    let a1978 = v0;
                    let a1979 = 20u64;
                    if nat_lt(a1978, a1979) {
                        let a1980 = v0;
                        let a1981 = 18u64;
                        if nat_lt(a1980, a1981) {
                            let a1982 = v0;
                            let a1983 = 17u64;
                            if nat_lt(a1982, a1983) {
                                Adt15::C0(1u64, 2u64, Adt13::C2, 13u64, 1u64)
                            } else {
                                Adt15::C0(1u64, 3u64, Adt13::C3, 13u64, 16u64)
                            }
                        } else {
                            let a1984 = v0;
                            let a1985 = 19u64;
                            if nat_lt(a1984, a1985) {
                                Adt15::C0(1u64, 4u64, Adt13::C0, 9u64, 13u64)
                            } else {
                                Adt15::C0(1u64, 5u64, Adt13::C2, 5u64, 9u64)
                            }
                        }
                    } else {
                        let a1986 = v0;
                        let a1987 = 22u64;
                        if nat_lt(a1986, a1987) {
                            let a1988 = v0;
                            let a1989 = 21u64;
                            if nat_lt(a1988, a1989) {
                                Adt15::C0(1u64, 6u64, Adt13::C3, 5u64, 12u64)
                            } else {
                                Adt15::C0(1u64, 7u64, Adt13::C0, 1u64, 5u64)
                            }
                        } else {
                            let a1990 = v0;
                            let a1991 = 23u64;
                            if nat_lt(a1990, a1991) {
                                Adt15::C0(1u64, 8u64, Adt13::C1, 1u64, 3u64)
                            } else {
                                Adt15::C0(1u64, 9u64, Adt13::C2, 13u64, 1u64)
                            }
                        }
                    }
                } else {
                    let a1992 = v0;
                    let a1993 = 28u64;
                    if nat_lt(a1992, a1993) {
                        let a1994 = v0;
                        let a1995 = 26u64;
                        if nat_lt(a1994, a1995) {
                            let a1996 = v0;
                            let a1997 = 25u64;
                            if nat_lt(a1996, a1997) {
                                Adt15::C0(1u64, 10u64, Adt13::C3, 13u64, 8u64)
                            } else {
                                Adt15::C0(1u64, 11u64, Adt13::C0, 9u64, 13u64)
                            }
                        } else {
                            let a1998 = v0;
                            let a1999 = 27u64;
                            if nat_lt(a1998, a1999) {
                                Adt15::C0(1u64, 12u64, Adt13::C2, 5u64, 9u64)
                            } else {
                                Adt15::C0(1u64, 13u64, Adt13::C3, 5u64, 7u64)
                            }
                        }
                    } else {
                        let a2000 = v0;
                        let a2001 = 30u64;
                        if nat_lt(a2000, a2001) {
                            let a2002 = v0;
                            let a2003 = 29u64;
                            if nat_lt(a2002, a2003) {
                                Adt15::C0(2u64, 0u64, Adt13::C0, 2u64, 6u64)
                            } else {
                                Adt15::C0(2u64, 1u64, Adt13::C1, 2u64, 4u64)
                            }
                        } else {
                            let a2004 = v0;
                            let a2005 = 31u64;
                            if nat_lt(a2004, a2005) {
                                Adt15::C0(2u64, 2u64, Adt13::C2, 14u64, 2u64)
                            } else {
                                Adt15::C0(2u64, 3u64, Adt13::C3, 14u64, 16u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a2006 = v0;
            let a2007 = 48u64;
            if nat_lt(a2006, a2007) {
                let a2008 = v0;
                let a2009 = 40u64;
                if nat_lt(a2008, a2009) {
                    let a2010 = v0;
                    let a2011 = 36u64;
                    if nat_lt(a2010, a2011) {
                        let a2012 = v0;
                        let a2013 = 34u64;
                        if nat_lt(a2012, a2013) {
                            let a2014 = v0;
                            let a2015 = 33u64;
                            if nat_lt(a2014, a2015) {
                                Adt15::C0(2u64, 4u64, Adt13::C0, 10u64, 14u64)
                            } else {
                                Adt15::C0(2u64, 5u64, Adt13::C2, 6u64, 10u64)
                            }
                        } else {
                            let a2016 = v0;
                            let a2017 = 35u64;
                            if nat_lt(a2016, a2017) {
                                Adt15::C0(2u64, 6u64, Adt13::C3, 6u64, 12u64)
                            } else {
                                Adt15::C0(2u64, 7u64, Adt13::C0, 2u64, 6u64)
                            }
                        }
                    } else {
                        let a2018 = v0;
                        let a2019 = 38u64;
                        if nat_lt(a2018, a2019) {
                            let a2020 = v0;
                            let a2021 = 37u64;
                            if nat_lt(a2020, a2021) {
                                Adt15::C0(2u64, 8u64, Adt13::C1, 2u64, 5u64)
                            } else {
                                Adt15::C0(2u64, 9u64, Adt13::C2, 14u64, 2u64)
                            }
                        } else {
                            let a2022 = v0;
                            let a2023 = 39u64;
                            if nat_lt(a2022, a2023) {
                                Adt15::C0(2u64, 10u64, Adt13::C3, 14u64, 8u64)
                            } else {
                                Adt15::C0(2u64, 11u64, Adt13::C0, 10u64, 14u64)
                            }
                        }
                    }
                } else {
                    let a2024 = v0;
                    let a2025 = 44u64;
                    if nat_lt(a2024, a2025) {
                        let a2026 = v0;
                        let a2027 = 42u64;
                        if nat_lt(a2026, a2027) {
                            let a2028 = v0;
                            let a2029 = 41u64;
                            if nat_lt(a2028, a2029) {
                                Adt15::C0(2u64, 12u64, Adt13::C2, 6u64, 10u64)
                            } else {
                                Adt15::C0(2u64, 13u64, Adt13::C3, 6u64, 7u64)
                            }
                        } else {
                            let a2030 = v0;
                            let a2031 = 43u64;
                            if nat_lt(a2030, a2031) {
                                Adt15::C0(3u64, 0u64, Adt13::C0, 3u64, 7u64)
                            } else {
                                Adt15::C0(3u64, 1u64, Adt13::C1, 3u64, 6u64)
                            }
                        }
                    } else {
                        let a2032 = v0;
                        let a2033 = 46u64;
                        if nat_lt(a2032, a2033) {
                            let a2034 = v0;
                            let a2035 = 45u64;
                            if nat_lt(a2034, a2035) {
                                Adt15::C0(3u64, 2u64, Adt13::C2, 15u64, 3u64)
                            } else {
                                Adt15::C0(3u64, 3u64, Adt13::C3, 15u64, 16u64)
                            }
                        } else {
                            let a2036 = v0;
                            let a2037 = 47u64;
                            if nat_lt(a2036, a2037) {
                                Adt15::C0(3u64, 4u64, Adt13::C0, 11u64, 15u64)
                            } else {
                                Adt15::C0(3u64, 5u64, Adt13::C2, 7u64, 11u64)
                            }
                        }
                    }
                }
            } else {
                let a2038 = v0;
                let a2039 = 56u64;
                if nat_lt(a2038, a2039) {
                    let a2040 = v0;
                    let a2041 = 52u64;
                    if nat_lt(a2040, a2041) {
                        let a2042 = v0;
                        let a2043 = 50u64;
                        if nat_lt(a2042, a2043) {
                            let a2044 = v0;
                            let a2045 = 49u64;
                            if nat_lt(a2044, a2045) {
                                Adt15::C0(3u64, 6u64, Adt13::C3, 7u64, 12u64)
                            } else {
                                Adt15::C0(3u64, 7u64, Adt13::C0, 3u64, 7u64)
                            }
                        } else {
                            let a2046 = v0;
                            let a2047 = 51u64;
                            if nat_lt(a2046, a2047) {
                                Adt15::C0(3u64, 8u64, Adt13::C1, 3u64, 7u64)
                            } else {
                                Adt15::C0(3u64, 9u64, Adt13::C2, 15u64, 3u64)
                            }
                        }
                    } else {
                        let a2048 = v0;
                        let a2049 = 54u64;
                        if nat_lt(a2048, a2049) {
                            let a2050 = v0;
                            let a2051 = 53u64;
                            if nat_lt(a2050, a2051) {
                                Adt15::C0(3u64, 10u64, Adt13::C3, 15u64, 8u64)
                            } else {
                                Adt15::C0(3u64, 11u64, Adt13::C0, 11u64, 15u64)
                            }
                        } else {
                            let a2052 = v0;
                            let a2053 = 55u64;
                            if nat_lt(a2052, a2053) {
                                Adt15::C0(3u64, 12u64, Adt13::C2, 7u64, 11u64)
                            } else {
                                Adt15::C0(3u64, 13u64, Adt13::C3, 7u64, 7u64)
                            }
                        }
                    }
                } else {
                    let a2054 = v0;
                    let a2055 = 60u64;
                    if nat_lt(a2054, a2055) {
                        let a2056 = v0;
                        let a2057 = 58u64;
                        if nat_lt(a2056, a2057) {
                            let a2058 = v0;
                            let a2059 = 57u64;
                            if nat_lt(a2058, a2059) {
                                Adt15::C0(4u64, 0u64, Adt13::C0, 0u64, 5u64)
                            } else {
                                Adt15::C0(4u64, 1u64, Adt13::C1, 0u64, 8u64)
                            }
                        } else {
                            let a2060 = v0;
                            let a2061 = 59u64;
                            if nat_lt(a2060, a2061) {
                                Adt15::C0(4u64, 2u64, Adt13::C2, 15u64, 0u64)
                            } else {
                                Adt15::C0(4u64, 3u64, Adt13::C3, 15u64, 16u64)
                            }
                        }
                    } else {
                        let a2062 = v0;
                        let a2063 = 62u64;
                        if nat_lt(a2062, a2063) {
                            let a2064 = v0;
                            let a2065 = 61u64;
                            if nat_lt(a2064, a2065) {
                                Adt15::C0(4u64, 4u64, Adt13::C0, 10u64, 15u64)
                            } else {
                                Adt15::C0(4u64, 5u64, Adt13::C2, 5u64, 10u64)
                            }
                        } else {
                            let a2066 = v0;
                            let a2067 = 63u64;
                            if nat_lt(a2066, a2067) {
                                Adt15::C0(4u64, 6u64, Adt13::C3, 5u64, 12u64)
                            } else {
                                Adt15::C0(4u64, 7u64, Adt13::C0, 0u64, 5u64)
                            }
                        }
                    }
                }
            }
        }
    } else {
        let a2068 = v0;
        let a2069 = 96u64;
        if nat_lt(a2068, a2069) {
            let a2070 = v0;
            let a2071 = 80u64;
            if nat_lt(a2070, a2071) {
                let a2072 = v0;
                let a2073 = 72u64;
                if nat_lt(a2072, a2073) {
                    let a2074 = v0;
                    let a2075 = 68u64;
                    if nat_lt(a2074, a2075) {
                        let a2076 = v0;
                        let a2077 = 66u64;
                        if nat_lt(a2076, a2077) {
                            let a2078 = v0;
                            let a2079 = 65u64;
                            if nat_lt(a2078, a2079) {
                                Adt15::C0(4u64, 8u64, Adt13::C1, 0u64, 9u64)
                            } else {
                                Adt15::C0(4u64, 9u64, Adt13::C2, 15u64, 0u64)
                            }
                        } else {
                            let a2080 = v0;
                            let a2081 = 67u64;
                            if nat_lt(a2080, a2081) {
                                Adt15::C0(4u64, 10u64, Adt13::C3, 15u64, 8u64)
                            } else {
                                Adt15::C0(4u64, 11u64, Adt13::C0, 10u64, 15u64)
                            }
                        }
                    } else {
                        let a2082 = v0;
                        let a2083 = 70u64;
                        if nat_lt(a2082, a2083) {
                            let a2084 = v0;
                            let a2085 = 69u64;
                            if nat_lt(a2084, a2085) {
                                Adt15::C0(4u64, 12u64, Adt13::C2, 5u64, 10u64)
                            } else {
                                Adt15::C0(4u64, 13u64, Adt13::C3, 5u64, 7u64)
                            }
                        } else {
                            let a2086 = v0;
                            let a2087 = 71u64;
                            if nat_lt(a2086, a2087) {
                                Adt15::C0(5u64, 0u64, Adt13::C0, 1u64, 6u64)
                            } else {
                                Adt15::C0(5u64, 1u64, Adt13::C1, 1u64, 10u64)
                            }
                        }
                    }
                } else {
                    let a2088 = v0;
                    let a2089 = 76u64;
                    if nat_lt(a2088, a2089) {
                        let a2090 = v0;
                        let a2091 = 74u64;
                        if nat_lt(a2090, a2091) {
                            let a2092 = v0;
                            let a2093 = 73u64;
                            if nat_lt(a2092, a2093) {
                                Adt15::C0(5u64, 2u64, Adt13::C2, 12u64, 1u64)
                            } else {
                                Adt15::C0(5u64, 3u64, Adt13::C3, 12u64, 16u64)
                            }
                        } else {
                            let a2094 = v0;
                            let a2095 = 75u64;
                            if nat_lt(a2094, a2095) {
                                Adt15::C0(5u64, 4u64, Adt13::C0, 11u64, 12u64)
                            } else {
                                Adt15::C0(5u64, 5u64, Adt13::C2, 6u64, 11u64)
                            }
                        }
                    } else {
                        let a2096 = v0;
                        let a2097 = 78u64;
                        if nat_lt(a2096, a2097) {
                            let a2098 = v0;
                            let a2099 = 77u64;
                            if nat_lt(a2098, a2099) {
                                Adt15::C0(5u64, 6u64, Adt13::C3, 6u64, 12u64)
                            } else {
                                Adt15::C0(5u64, 7u64, Adt13::C0, 1u64, 6u64)
                            }
                        } else {
                            let a2100 = v0;
                            let a2101 = 79u64;
                            if nat_lt(a2100, a2101) {
                                Adt15::C0(5u64, 8u64, Adt13::C1, 1u64, 11u64)
                            } else {
                                Adt15::C0(5u64, 9u64, Adt13::C2, 12u64, 1u64)
                            }
                        }
                    }
                }
            } else {
                let a2102 = v0;
                let a2103 = 88u64;
                if nat_lt(a2102, a2103) {
                    let a2104 = v0;
                    let a2105 = 84u64;
                    if nat_lt(a2104, a2105) {
                        let a2106 = v0;
                        let a2107 = 82u64;
                        if nat_lt(a2106, a2107) {
                            let a2108 = v0;
                            let a2109 = 81u64;
                            if nat_lt(a2108, a2109) {
                                Adt15::C0(5u64, 10u64, Adt13::C3, 12u64, 8u64)
                            } else {
                                Adt15::C0(5u64, 11u64, Adt13::C0, 11u64, 12u64)
                            }
                        } else {
                            let a2110 = v0;
                            let a2111 = 83u64;
                            if nat_lt(a2110, a2111) {
                                Adt15::C0(5u64, 12u64, Adt13::C2, 6u64, 11u64)
                            } else {
                                Adt15::C0(5u64, 13u64, Adt13::C3, 6u64, 7u64)
                            }
                        }
                    } else {
                        let a2112 = v0;
                        let a2113 = 86u64;
                        if nat_lt(a2112, a2113) {
                            let a2114 = v0;
                            let a2115 = 85u64;
                            if nat_lt(a2114, a2115) {
                                Adt15::C0(6u64, 0u64, Adt13::C0, 2u64, 7u64)
                            } else {
                                Adt15::C0(6u64, 1u64, Adt13::C1, 2u64, 12u64)
                            }
                        } else {
                            let a2116 = v0;
                            let a2117 = 87u64;
                            if nat_lt(a2116, a2117) {
                                Adt15::C0(6u64, 2u64, Adt13::C2, 13u64, 2u64)
                            } else {
                                Adt15::C0(6u64, 3u64, Adt13::C3, 13u64, 16u64)
                            }
                        }
                    }
                } else {
                    let a2118 = v0;
                    let a2119 = 92u64;
                    if nat_lt(a2118, a2119) {
                        let a2120 = v0;
                        let a2121 = 90u64;
                        if nat_lt(a2120, a2121) {
                            let a2122 = v0;
                            let a2123 = 89u64;
                            if nat_lt(a2122, a2123) {
                                Adt15::C0(6u64, 4u64, Adt13::C0, 8u64, 13u64)
                            } else {
                                Adt15::C0(6u64, 5u64, Adt13::C2, 7u64, 8u64)
                            }
                        } else {
                            let a2124 = v0;
                            let a2125 = 91u64;
                            if nat_lt(a2124, a2125) {
                                Adt15::C0(6u64, 6u64, Adt13::C3, 7u64, 12u64)
                            } else {
                                Adt15::C0(6u64, 7u64, Adt13::C0, 2u64, 7u64)
                            }
                        }
                    } else {
                        let a2126 = v0;
                        let a2127 = 94u64;
                        if nat_lt(a2126, a2127) {
                            let a2128 = v0;
                            let a2129 = 93u64;
                            if nat_lt(a2128, a2129) {
                                Adt15::C0(6u64, 8u64, Adt13::C1, 2u64, 13u64)
                            } else {
                                Adt15::C0(6u64, 9u64, Adt13::C2, 13u64, 2u64)
                            }
                        } else {
                            let a2130 = v0;
                            let a2131 = 95u64;
                            if nat_lt(a2130, a2131) {
                                Adt15::C0(6u64, 10u64, Adt13::C3, 13u64, 8u64)
                            } else {
                                Adt15::C0(6u64, 11u64, Adt13::C0, 8u64, 13u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a2132 = v0;
            let a2133 = 112u64;
            if nat_lt(a2132, a2133) {
                let a2134 = v0;
                let a2135 = 104u64;
                if nat_lt(a2134, a2135) {
                    let a2136 = v0;
                    let a2137 = 100u64;
                    if nat_lt(a2136, a2137) {
                        let a2138 = v0;
                        let a2139 = 98u64;
                        if nat_lt(a2138, a2139) {
                            let a2140 = v0;
                            let a2141 = 97u64;
                            if nat_lt(a2140, a2141) {
                                Adt15::C0(6u64, 12u64, Adt13::C2, 7u64, 8u64)
                            } else {
                                Adt15::C0(6u64, 13u64, Adt13::C3, 7u64, 7u64)
                            }
                        } else {
                            let a2142 = v0;
                            let a2143 = 99u64;
                            if nat_lt(a2142, a2143) {
                                Adt15::C0(7u64, 0u64, Adt13::C0, 3u64, 4u64)
                            } else {
                                Adt15::C0(7u64, 1u64, Adt13::C1, 3u64, 14u64)
                            }
                        }
                    } else {
                        let a2144 = v0;
                        let a2145 = 102u64;
                        if nat_lt(a2144, a2145) {
                            let a2146 = v0;
                            let a2147 = 101u64;
                            if nat_lt(a2146, a2147) {
                                Adt15::C0(7u64, 2u64, Adt13::C2, 14u64, 3u64)
                            } else {
                                Adt15::C0(7u64, 3u64, Adt13::C3, 14u64, 16u64)
                            }
                        } else {
                            let a2148 = v0;
                            let a2149 = 103u64;
                            if nat_lt(a2148, a2149) {
                                Adt15::C0(7u64, 4u64, Adt13::C0, 9u64, 14u64)
                            } else {
                                Adt15::C0(7u64, 5u64, Adt13::C2, 4u64, 9u64)
                            }
                        }
                    }
                } else {
                    let a2150 = v0;
                    let a2151 = 108u64;
                    if nat_lt(a2150, a2151) {
                        let a2152 = v0;
                        let a2153 = 106u64;
                        if nat_lt(a2152, a2153) {
                            let a2154 = v0;
                            let a2155 = 105u64;
                            if nat_lt(a2154, a2155) {
                                Adt15::C0(7u64, 6u64, Adt13::C3, 4u64, 12u64)
                            } else {
                                Adt15::C0(7u64, 7u64, Adt13::C0, 3u64, 4u64)
                            }
                        } else {
                            let a2156 = v0;
                            let a2157 = 107u64;
                            if nat_lt(a2156, a2157) {
                                Adt15::C0(7u64, 8u64, Adt13::C1, 3u64, 15u64)
                            } else {
                                Adt15::C0(7u64, 9u64, Adt13::C2, 14u64, 3u64)
                            }
                        }
                    } else {
                        let a2158 = v0;
                        let a2159 = 110u64;
                        if nat_lt(a2158, a2159) {
                            let a2160 = v0;
                            let a2161 = 109u64;
                            if nat_lt(a2160, a2161) {
                                Adt15::C0(7u64, 10u64, Adt13::C3, 14u64, 8u64)
                            } else {
                                Adt15::C0(7u64, 11u64, Adt13::C0, 9u64, 14u64)
                            }
                        } else {
                            let a2162 = v0;
                            let a2163 = 111u64;
                            if nat_lt(a2162, a2163) {
                                Adt15::C0(7u64, 12u64, Adt13::C2, 4u64, 9u64)
                            } else {
                                Adt15::C0(7u64, 13u64, Adt13::C3, 4u64, 7u64)
                            }
                        }
                    }
                }
            } else {
                let a2164 = v0;
                let a2165 = 120u64;
                if nat_lt(a2164, a2165) {
                    let a2166 = v0;
                    let a2167 = 116u64;
                    if nat_lt(a2166, a2167) {
                        let a2168 = v0;
                        let a2169 = 114u64;
                        if nat_lt(a2168, a2169) {
                            let a2170 = v0;
                            let a2171 = 113u64;
                            if nat_lt(a2170, a2171) {
                                Adt15::C0(0u64, 0u64, Adt13::C2, 0u64, 8u64)
                            } else {
                                Adt15::C0(0u64, 1u64, Adt13::C2, 1u64, 9u64)
                            }
                        } else {
                            let a2172 = v0;
                            let a2173 = 115u64;
                            if nat_lt(a2172, a2173) {
                                Adt15::C0(0u64, 2u64, Adt13::C2, 2u64, 10u64)
                            } else {
                                Adt15::C0(0u64, 3u64, Adt13::C2, 3u64, 11u64)
                            }
                        }
                    } else {
                        let a2174 = v0;
                        let a2175 = 118u64;
                        if nat_lt(a2174, a2175) {
                            let a2176 = v0;
                            let a2177 = 117u64;
                            if nat_lt(a2176, a2177) {
                                Adt15::C0(0u64, 4u64, Adt13::C2, 4u64, 12u64)
                            } else {
                                Adt15::C0(0u64, 5u64, Adt13::C2, 5u64, 13u64)
                            }
                        } else {
                            let a2178 = v0;
                            let a2179 = 119u64;
                            if nat_lt(a2178, a2179) {
                                Adt15::C0(0u64, 6u64, Adt13::C2, 6u64, 14u64)
                            } else {
                                Adt15::C0(0u64, 7u64, Adt13::C2, 7u64, 15u64)
                            }
                        }
                    }
                } else {
                    let a2180 = v0;
                    let a2181 = 124u64;
                    if nat_lt(a2180, a2181) {
                        let a2182 = v0;
                        let a2183 = 122u64;
                        if nat_lt(a2182, a2183) {
                            let a2184 = v0;
                            let a2185 = 121u64;
                            if nat_lt(a2184, a2185) {
                                Adt15::C0(0u64, 8u64, Adt13::C4, 8u64, 0u64)
                            } else {
                                Adt15::C0(0u64, 9u64, Adt13::C4, 9u64, 1u64)
                            }
                        } else {
                            let a2186 = v0;
                            let a2187 = 123u64;
                            if nat_lt(a2186, a2187) {
                                Adt15::C0(0u64, 10u64, Adt13::C4, 10u64, 2u64)
                            } else {
                                Adt15::C0(0u64, 11u64, Adt13::C4, 11u64, 3u64)
                            }
                        }
                    } else {
                        let a2188 = v0;
                        let a2189 = 126u64;
                        if nat_lt(a2188, a2189) {
                            let a2190 = v0;
                            let a2191 = 125u64;
                            if nat_lt(a2190, a2191) {
                                Adt15::C0(0u64, 12u64, Adt13::C4, 12u64, 4u64)
                            } else {
                                Adt15::C0(0u64, 13u64, Adt13::C4, 13u64, 5u64)
                            }
                        } else {
                            let a2192 = v0;
                            let a2193 = 127u64;
                            if nat_lt(a2192, a2193) {
                                Adt15::C0(0u64, 14u64, Adt13::C4, 14u64, 6u64)
                            } else {
                                Adt15::C0(0u64, 15u64, Adt13::C4, 15u64, 7u64)
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn f96(v0: Adt4, v1: u64) -> u32 {
    let a2194 = v1;
    let a2195 = 8u64;
    if nat_lt(a2194, a2195) {
        let a2196 = v1;
        let a2197 = 4u64;
        if nat_lt(a2196, a2197) {
            let a2198 = v1;
            let a2199 = 2u64;
            if nat_lt(a2198, a2199) {
                let a2200 = v1;
                let a2201 = 1u64;
                if nat_lt(a2200, a2201) {
                    match v0 {
                        Adt4::C0(h2202, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2202
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, h2203, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2203
                        }
                    }
                }
            } else {
                let a2204 = v1;
                let a2205 = 3u64;
                if nat_lt(a2204, a2205) {
                    match v0 {
                        Adt4::C0(_, _, h2206, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2206
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, h2207, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2207
                        }
                    }
                }
            }
        } else {
            let a2208 = v1;
            let a2209 = 6u64;
            if nat_lt(a2208, a2209) {
                let a2210 = v1;
                let a2211 = 5u64;
                if nat_lt(a2210, a2211) {
                    match v0 {
                        Adt4::C0(_, _, _, _, h2212, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2212
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, h2213, _, _, _, _, _, _, _, _, _, _) => {
                            h2213
                        }
                    }
                }
            } else {
                let a2214 = v1;
                let a2215 = 7u64;
                if nat_lt(a2214, a2215) {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, h2216, _, _, _, _, _, _, _, _, _) => {
                            h2216
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, h2217, _, _, _, _, _, _, _, _) => {
                            h2217
                        }
                    }
                }
            }
        }
    } else {
        let a2218 = v1;
        let a2219 = 12u64;
        if nat_lt(a2218, a2219) {
            let a2220 = v1;
            let a2221 = 10u64;
            if nat_lt(a2220, a2221) {
                let a2222 = v1;
                let a2223 = 9u64;
                if nat_lt(a2222, a2223) {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, h2224, _, _, _, _, _, _, _) => {
                            h2224
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h2225, _, _, _, _, _, _) => {
                            h2225
                        }
                    }
                }
            } else {
                let a2226 = v1;
                let a2227 = 11u64;
                if nat_lt(a2226, a2227) {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h2228, _, _, _, _, _) => {
                            h2228
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h2229, _, _, _, _) => {
                            h2229
                        }
                    }
                }
            }
        } else {
            let a2230 = v1;
            let a2231 = 14u64;
            if nat_lt(a2230, a2231) {
                let a2232 = v1;
                let a2233 = 13u64;
                if nat_lt(a2232, a2233) {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2234, _, _, _) => {
                            h2234
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2235, _, _) => {
                            h2235
                        }
                    }
                }
            } else {
                let a2236 = v1;
                let a2237 = 15u64;
                if nat_lt(a2236, a2237) {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2238, _) => {
                            h2238
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2239) => {
                            h2239
                        }
                    }
                }
            }
        }
    }
}

pub fn f97(v0: u64, v1: u64) -> R<u64> {
    let v2: u64 = {
        let a2242 = {
            let a2240 = v0;
            let a2241 = 16u64;
            nat_mul(a2240, a2241)?
        };
        let a2243 = v1;
        nat_add(a2242, a2243)?
    };
    let a2244 = v2;
    let a2245 = 56u64;
    if nat_lt(a2244, a2245) {
        let a2246 = v2;
        let a2247 = 28u64;
        if nat_lt(a2246, a2247) {
            let a2248 = v2;
            let a2249 = 14u64;
            if nat_lt(a2248, a2249) {
                let a2250 = v2;
                let a2251 = 7u64;
                if nat_lt(a2250, a2251) {
                    let a2252 = v2;
                    let a2253 = 3u64;
                    if nat_lt(a2252, a2253) {
                        let a2254 = v2;
                        let a2255 = 1u64;
                        if nat_lt(a2254, a2255) {
                            Ok(0u64)
                        } else {
                            let a2256 = v2;
                            let a2257 = 2u64;
                            if nat_lt(a2256, a2257) {
                                Ok(1u64)
                            } else {
                                Ok(2u64)
                            }
                        }
                    } else {
                        let a2258 = v2;
                        let a2259 = 5u64;
                        if nat_lt(a2258, a2259) {
                            let a2260 = v2;
                            let a2261 = 4u64;
                            if nat_lt(a2260, a2261) {
                                Ok(3u64)
                            } else {
                                Ok(4u64)
                            }
                        } else {
                            let a2262 = v2;
                            let a2263 = 6u64;
                            if nat_lt(a2262, a2263) {
                                Ok(5u64)
                            } else {
                                Ok(6u64)
                            }
                        }
                    }
                } else {
                    let a2264 = v2;
                    let a2265 = 10u64;
                    if nat_lt(a2264, a2265) {
                        let a2266 = v2;
                        let a2267 = 8u64;
                        if nat_lt(a2266, a2267) {
                            Ok(7u64)
                        } else {
                            let a2268 = v2;
                            let a2269 = 9u64;
                            if nat_lt(a2268, a2269) {
                                Ok(8u64)
                            } else {
                                Ok(9u64)
                            }
                        }
                    } else {
                        let a2270 = v2;
                        let a2271 = 12u64;
                        if nat_lt(a2270, a2271) {
                            let a2272 = v2;
                            let a2273 = 11u64;
                            if nat_lt(a2272, a2273) {
                                Ok(10u64)
                            } else {
                                Ok(11u64)
                            }
                        } else {
                            let a2274 = v2;
                            let a2275 = 13u64;
                            if nat_lt(a2274, a2275) {
                                Ok(12u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    }
                }
            } else {
                let a2276 = v2;
                let a2277 = 21u64;
                if nat_lt(a2276, a2277) {
                    let a2278 = v2;
                    let a2279 = 17u64;
                    if nat_lt(a2278, a2279) {
                        let a2280 = v2;
                        let a2281 = 15u64;
                        if nat_lt(a2280, a2281) {
                            Ok(14u64)
                        } else {
                            let a2282 = v2;
                            let a2283 = 16u64;
                            if nat_lt(a2282, a2283) {
                                Ok(15u64)
                            } else {
                                Ok(2u64)
                            }
                        }
                    } else {
                        let a2284 = v2;
                        let a2285 = 19u64;
                        if nat_lt(a2284, a2285) {
                            let a2286 = v2;
                            let a2287 = 18u64;
                            if nat_lt(a2286, a2287) {
                                Ok(6u64)
                            } else {
                                Ok(3u64)
                            }
                        } else {
                            let a2288 = v2;
                            let a2289 = 20u64;
                            if nat_lt(a2288, a2289) {
                                Ok(10u64)
                            } else {
                                Ok(7u64)
                            }
                        }
                    }
                } else {
                    let a2290 = v2;
                    let a2291 = 24u64;
                    if nat_lt(a2290, a2291) {
                        let a2292 = v2;
                        let a2293 = 22u64;
                        if nat_lt(a2292, a2293) {
                            Ok(0u64)
                        } else {
                            let a2294 = v2;
                            let a2295 = 23u64;
                            if nat_lt(a2294, a2295) {
                                Ok(4u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    } else {
                        let a2296 = v2;
                        let a2297 = 26u64;
                        if nat_lt(a2296, a2297) {
                            let a2298 = v2;
                            let a2299 = 25u64;
                            if nat_lt(a2298, a2299) {
                                Ok(1u64)
                            } else {
                                Ok(11u64)
                            }
                        } else {
                            let a2300 = v2;
                            let a2301 = 27u64;
                            if nat_lt(a2300, a2301) {
                                Ok(12u64)
                            } else {
                                Ok(5u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a2302 = v2;
            let a2303 = 42u64;
            if nat_lt(a2302, a2303) {
                let a2304 = v2;
                let a2305 = 35u64;
                if nat_lt(a2304, a2305) {
                    let a2306 = v2;
                    let a2307 = 31u64;
                    if nat_lt(a2306, a2307) {
                        let a2308 = v2;
                        let a2309 = 29u64;
                        if nat_lt(a2308, a2309) {
                            Ok(9u64)
                        } else {
                            let a2310 = v2;
                            let a2311 = 30u64;
                            if nat_lt(a2310, a2311) {
                                Ok(14u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    } else {
                        let a2312 = v2;
                        let a2313 = 33u64;
                        if nat_lt(a2312, a2313) {
                            let a2314 = v2;
                            let a2315 = 32u64;
                            if nat_lt(a2314, a2315) {
                                Ok(8u64)
                            } else {
                                Ok(3u64)
                            }
                        } else {
                            let a2316 = v2;
                            let a2317 = 34u64;
                            if nat_lt(a2316, a2317) {
                                Ok(4u64)
                            } else {
                                Ok(10u64)
                            }
                        }
                    }
                } else {
                    let a2318 = v2;
                    let a2319 = 38u64;
                    if nat_lt(a2318, a2319) {
                        let a2320 = v2;
                        let a2321 = 36u64;
                        if nat_lt(a2320, a2321) {
                            Ok(12u64)
                        } else {
                            let a2322 = v2;
                            let a2323 = 37u64;
                            if nat_lt(a2322, a2323) {
                                Ok(13u64)
                            } else {
                                Ok(2u64)
                            }
                        }
                    } else {
                        let a2324 = v2;
                        let a2325 = 40u64;
                        if nat_lt(a2324, a2325) {
                            let a2326 = v2;
                            let a2327 = 39u64;
                            if nat_lt(a2326, a2327) {
                                Ok(7u64)
                            } else {
                                Ok(14u64)
                            }
                        } else {
                            let a2328 = v2;
                            let a2329 = 41u64;
                            if nat_lt(a2328, a2329) {
                                Ok(6u64)
                            } else {
                                Ok(5u64)
                            }
                        }
                    }
                }
            } else {
                let a2330 = v2;
                let a2331 = 49u64;
                if nat_lt(a2330, a2331) {
                    let a2332 = v2;
                    let a2333 = 45u64;
                    if nat_lt(a2332, a2333) {
                        let a2334 = v2;
                        let a2335 = 43u64;
                        if nat_lt(a2334, a2335) {
                            Ok(9u64)
                        } else {
                            let a2336 = v2;
                            let a2337 = 44u64;
                            if nat_lt(a2336, a2337) {
                                Ok(0u64)
                            } else {
                                Ok(11u64)
                            }
                        }
                    } else {
                        let a2338 = v2;
                        let a2339 = 47u64;
                        if nat_lt(a2338, a2339) {
                            let a2340 = v2;
                            let a2341 = 46u64;
                            if nat_lt(a2340, a2341) {
                                Ok(15u64)
                            } else {
                                Ok(8u64)
                            }
                        } else {
                            let a2342 = v2;
                            let a2343 = 48u64;
                            if nat_lt(a2342, a2343) {
                                Ok(1u64)
                            } else {
                                Ok(10u64)
                            }
                        }
                    }
                } else {
                    let a2344 = v2;
                    let a2345 = 52u64;
                    if nat_lt(a2344, a2345) {
                        let a2346 = v2;
                        let a2347 = 50u64;
                        if nat_lt(a2346, a2347) {
                            Ok(7u64)
                        } else {
                            let a2348 = v2;
                            let a2349 = 51u64;
                            if nat_lt(a2348, a2349) {
                                Ok(12u64)
                            } else {
                                Ok(9u64)
                            }
                        }
                    } else {
                        let a2350 = v2;
                        let a2351 = 54u64;
                        if nat_lt(a2350, a2351) {
                            let a2352 = v2;
                            let a2353 = 53u64;
                            if nat_lt(a2352, a2353) {
                                Ok(14u64)
                            } else {
                                Ok(3u64)
                            }
                        } else {
                            let a2354 = v2;
                            let a2355 = 55u64;
                            if nat_lt(a2354, a2355) {
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
        let a2356 = v2;
        let a2357 = 84u64;
        if nat_lt(a2356, a2357) {
            let a2358 = v2;
            let a2359 = 70u64;
            if nat_lt(a2358, a2359) {
                let a2360 = v2;
                let a2361 = 63u64;
                if nat_lt(a2360, a2361) {
                    let a2362 = v2;
                    let a2363 = 59u64;
                    if nat_lt(a2362, a2363) {
                        let a2364 = v2;
                        let a2365 = 57u64;
                        if nat_lt(a2364, a2365) {
                            Ok(4u64)
                        } else {
                            let a2366 = v2;
                            let a2367 = 58u64;
                            if nat_lt(a2366, a2367) {
                                Ok(0u64)
                            } else {
                                Ok(11u64)
                            }
                        }
                    } else {
                        let a2368 = v2;
                        let a2369 = 61u64;
                        if nat_lt(a2368, a2369) {
                            let a2370 = v2;
                            let a2371 = 60u64;
                            if nat_lt(a2370, a2371) {
                                Ok(2u64)
                            } else {
                                Ok(5u64)
                            }
                        } else {
                            let a2372 = v2;
                            let a2373 = 62u64;
                            if nat_lt(a2372, a2373) {
                                Ok(8u64)
                            } else {
                                Ok(1u64)
                            }
                        }
                    }
                } else {
                    let a2374 = v2;
                    let a2375 = 66u64;
                    if nat_lt(a2374, a2375) {
                        let a2376 = v2;
                        let a2377 = 64u64;
                        if nat_lt(a2376, a2377) {
                            Ok(6u64)
                        } else {
                            let a2378 = v2;
                            let a2379 = 65u64;
                            if nat_lt(a2378, a2379) {
                                Ok(12u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    } else {
                        let a2380 = v2;
                        let a2381 = 68u64;
                        if nat_lt(a2380, a2381) {
                            let a2382 = v2;
                            let a2383 = 67u64;
                            if nat_lt(a2382, a2383) {
                                Ok(9u64)
                            } else {
                                Ok(11u64)
                            }
                        } else {
                            let a2384 = v2;
                            let a2385 = 69u64;
                            if nat_lt(a2384, a2385) {
                                Ok(15u64)
                            } else {
                                Ok(10u64)
                            }
                        }
                    }
                }
            } else {
                let a2386 = v2;
                let a2387 = 77u64;
                if nat_lt(a2386, a2387) {
                    let a2388 = v2;
                    let a2389 = 73u64;
                    if nat_lt(a2388, a2389) {
                        let a2390 = v2;
                        let a2391 = 71u64;
                        if nat_lt(a2390, a2391) {
                            Ok(14u64)
                        } else {
                            let a2392 = v2;
                            let a2393 = 72u64;
                            if nat_lt(a2392, a2393) {
                                Ok(8u64)
                            } else {
                                Ok(7u64)
                            }
                        }
                    } else {
                        let a2394 = v2;
                        let a2395 = 75u64;
                        if nat_lt(a2394, a2395) {
                            let a2396 = v2;
                            let a2397 = 74u64;
                            if nat_lt(a2396, a2397) {
                                Ok(2u64)
                            } else {
                                Ok(5u64)
                            }
                        } else {
                            let a2398 = v2;
                            let a2399 = 76u64;
                            if nat_lt(a2398, a2399) {
                                Ok(3u64)
                            } else {
                                Ok(0u64)
                            }
                        }
                    }
                } else {
                    let a2400 = v2;
                    let a2401 = 80u64;
                    if nat_lt(a2400, a2401) {
                        let a2402 = v2;
                        let a2403 = 78u64;
                        if nat_lt(a2402, a2403) {
                            Ok(1u64)
                        } else {
                            let a2404 = v2;
                            let a2405 = 79u64;
                            if nat_lt(a2404, a2405) {
                                Ok(6u64)
                            } else {
                                Ok(4u64)
                            }
                        }
                    } else {
                        let a2406 = v2;
                        let a2407 = 82u64;
                        if nat_lt(a2406, a2407) {
                            let a2408 = v2;
                            let a2409 = 81u64;
                            if nat_lt(a2408, a2409) {
                                Ok(9u64)
                            } else {
                                Ok(14u64)
                            }
                        } else {
                            let a2410 = v2;
                            let a2411 = 83u64;
                            if nat_lt(a2410, a2411) {
                                Ok(11u64)
                            } else {
                                Ok(5u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a2412 = v2;
            let a2413 = 98u64;
            if nat_lt(a2412, a2413) {
                let a2414 = v2;
                let a2415 = 91u64;
                if nat_lt(a2414, a2415) {
                    let a2416 = v2;
                    let a2417 = 87u64;
                    if nat_lt(a2416, a2417) {
                        let a2418 = v2;
                        let a2419 = 85u64;
                        if nat_lt(a2418, a2419) {
                            Ok(8u64)
                        } else {
                            let a2420 = v2;
                            let a2421 = 86u64;
                            if nat_lt(a2420, a2421) {
                                Ok(12u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    } else {
                        let a2422 = v2;
                        let a2423 = 89u64;
                        if nat_lt(a2422, a2423) {
                            let a2424 = v2;
                            let a2425 = 88u64;
                            if nat_lt(a2424, a2425) {
                                Ok(1u64)
                            } else {
                                Ok(13u64)
                            }
                        } else {
                            let a2426 = v2;
                            let a2427 = 90u64;
                            if nat_lt(a2426, a2427) {
                                Ok(3u64)
                            } else {
                                Ok(0u64)
                            }
                        }
                    }
                } else {
                    let a2428 = v2;
                    let a2429 = 94u64;
                    if nat_lt(a2428, a2429) {
                        let a2430 = v2;
                        let a2431 = 92u64;
                        if nat_lt(a2430, a2431) {
                            Ok(10u64)
                        } else {
                            let a2432 = v2;
                            let a2433 = 93u64;
                            if nat_lt(a2432, a2433) {
                                Ok(2u64)
                            } else {
                                Ok(6u64)
                            }
                        }
                    } else {
                        let a2434 = v2;
                        let a2435 = 96u64;
                        if nat_lt(a2434, a2435) {
                            let a2436 = v2;
                            let a2437 = 95u64;
                            if nat_lt(a2436, a2437) {
                                Ok(4u64)
                            } else {
                                Ok(7u64)
                            }
                        } else {
                            let a2438 = v2;
                            let a2439 = 97u64;
                            if nat_lt(a2438, a2439) {
                                Ok(11u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    }
                }
            } else {
                let a2440 = v2;
                let a2441 = 105u64;
                if nat_lt(a2440, a2441) {
                    let a2442 = v2;
                    let a2443 = 101u64;
                    if nat_lt(a2442, a2443) {
                        let a2444 = v2;
                        let a2445 = 99u64;
                        if nat_lt(a2444, a2445) {
                            Ok(5u64)
                        } else {
                            let a2446 = v2;
                            let a2447 = 100u64;
                            if nat_lt(a2446, a2447) {
                                Ok(0u64)
                            } else {
                                Ok(1u64)
                            }
                        }
                    } else {
                        let a2448 = v2;
                        let a2449 = 103u64;
                        if nat_lt(a2448, a2449) {
                            let a2450 = v2;
                            let a2451 = 102u64;
                            if nat_lt(a2450, a2451) {
                                Ok(9u64)
                            } else {
                                Ok(8u64)
                            }
                        } else {
                            let a2452 = v2;
                            let a2453 = 104u64;
                            if nat_lt(a2452, a2453) {
                                Ok(6u64)
                            } else {
                                Ok(14u64)
                            }
                        }
                    }
                } else {
                    let a2454 = v2;
                    let a2455 = 108u64;
                    if nat_lt(a2454, a2455) {
                        let a2456 = v2;
                        let a2457 = 106u64;
                        if nat_lt(a2456, a2457) {
                            Ok(10u64)
                        } else {
                            let a2458 = v2;
                            let a2459 = 107u64;
                            if nat_lt(a2458, a2459) {
                                Ok(2u64)
                            } else {
                                Ok(12u64)
                            }
                        }
                    } else {
                        let a2460 = v2;
                        let a2461 = 110u64;
                        if nat_lt(a2460, a2461) {
                            let a2462 = v2;
                            let a2463 = 109u64;
                            if nat_lt(a2462, a2463) {
                                Ok(3u64)
                            } else {
                                Ok(4u64)
                            }
                        } else {
                            let a2464 = v2;
                            let a2465 = 111u64;
                            if nat_lt(a2464, a2465) {
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

pub fn f98(v0: u64) -> u32 {
    let a2466 = v0;
    let a2467 = 12u64;
    if nat_lt(a2466, a2467) {
        let a2468 = v0;
        let a2469 = 9u64;
        if nat_lt(a2468, a2469) {
            let a2470 = v0;
            let a2471 = 8u64;
            if nat_lt(a2470, a2471) {
                7u32
            } else {
                8u32
            }
        } else {
            let a2472 = v0;
            let a2473 = 10u64;
            if nat_lt(a2472, a2473) {
                9u32
            } else {
                let a2474 = v0;
                let a2475 = 11u64;
                if nat_lt(a2474, a2475) {
                    10u32
                } else {
                    11u32
                }
            }
        }
    } else {
        let a2476 = v0;
        let a2477 = 14u64;
        if nat_lt(a2476, a2477) {
            let a2478 = v0;
            let a2479 = 13u64;
            if nat_lt(a2478, a2479) {
                12u32
            } else {
                13u32
            }
        } else {
            let a2480 = v0;
            let a2481 = 15u64;
            if nat_lt(a2480, a2481) {
                14u32
            } else {
                let a2482 = v0;
                let a2483 = 16u64;
                if nat_lt(a2482, a2483) {
                    15u32
                } else {
                    16u32
                }
            }
        }
    }
}

pub fn f99(v0: Adt3, v1: u64) -> u32 {
    let a2484 = v1;
    let a2485 = 4u64;
    if nat_lt(a2484, a2485) {
        let a2486 = v1;
        let a2487 = 2u64;
        if nat_lt(a2486, a2487) {
            let a2488 = v1;
            let a2489 = 1u64;
            if nat_lt(a2488, a2489) {
                match v0 {
                    Adt3::C0(h2490, _, _, _, _, _, _, _) => {
                        h2490
                    }
                }
            } else {
                match v0 {
                    Adt3::C0(_, h2491, _, _, _, _, _, _) => {
                        h2491
                    }
                }
            }
        } else {
            let a2492 = v1;
            let a2493 = 3u64;
            if nat_lt(a2492, a2493) {
                match v0 {
                    Adt3::C0(_, _, h2494, _, _, _, _, _) => {
                        h2494
                    }
                }
            } else {
                match v0 {
                    Adt3::C0(_, _, _, h2495, _, _, _, _) => {
                        h2495
                    }
                }
            }
        }
    } else {
        let a2496 = v1;
        let a2497 = 6u64;
        if nat_lt(a2496, a2497) {
            let a2498 = v1;
            let a2499 = 5u64;
            if nat_lt(a2498, a2499) {
                match v0 {
                    Adt3::C0(_, _, _, _, h2500, _, _, _) => {
                        h2500
                    }
                }
            } else {
                match v0 {
                    Adt3::C0(_, _, _, _, _, h2501, _, _) => {
                        h2501
                    }
                }
            }
        } else {
            let a2502 = v1;
            let a2503 = 7u64;
            if nat_lt(a2502, a2503) {
                match v0 {
                    Adt3::C0(_, _, _, _, _, _, h2504, _) => {
                        h2504
                    }
                }
            } else {
                match v0 {
                    Adt3::C0(_, _, _, _, _, _, _, h2505) => {
                        h2505
                    }
                }
            }
        }
    }
}

pub fn f100(v0: Adt13, v1: u32, v2: u32) -> u32 {
    let m2506 = v0;
    match m2506 {
        Adt13::C0 => {
            f101(v1, v2)
        }
        Adt13::C1 => {
            f101(v1, v2)
        }
        Adt13::C2 => {
            f102(v1, v2)
        }
        Adt13::C3 => {
            f103(v1, v2)
        }
        Adt13::C4 => {
            f102(v1, v2)
        }
    }
}

pub fn f101(v0: u32, v1: u32) -> u32 {
    let m2509 = {
        let a2507 = f104(v0);
        let a2508 = f104(v1);
        fixed_u64::checked_add(a2507, a2508)
    };
    match m2509 {
        None => {
            0u32
        }
        Some(v2) => {
            f105(v2)
        }
    }
}

pub fn f102(v0: u32, v1: u32) -> u32 {
    let a2510 = v0;
    let a2511 = v1;
    fixed_u32::bit_xor(a2510, a2511)
}

pub fn f103(v0: u32, v1: u32) -> u32 {
    let m2514 = {
        let a2512 = 32u32;
        let a2513 = v1;
        fixed_u32::checked_sub(a2512, a2513)
    };
    match m2514 {
        None => {
            v0
        }
        Some(v2) => {
            f106(v0, v1, v2)
        }
    }
}

pub fn f104(v0: u32) -> u64 {
    let m2516 = {
        let a2515 = v0;
        fixed_u64::convert(i128::from(a2515))
    };
    match m2516 {
        None => {
            0u64
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f105(v0: u64) -> u32 {
    let m2520 = {
        let a2519 = {
            let a2517 = v0;
            let a2518 = 4294967295u64;
            fixed_u64::bit_and(a2517, a2518)
        };
        fixed_u32::convert(i128::from(a2519))
    };
    match m2520 {
        None => {
            0u32
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f106(v0: u32, v1: u32, v2: u32) -> u32 {
    let a2521 = f41(v0, v1);
    let a2522 = f80(v0, v2);
    fixed_u32::bit_or(a2521, a2522)
}

pub fn run(p0: Adt0, p1: Bytes, p2: u64) -> R<Result<Bytes, Adt1>> {
    f0(p0, p1, p2)
}
