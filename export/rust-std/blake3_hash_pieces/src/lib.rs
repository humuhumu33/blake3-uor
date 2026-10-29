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
    C0(u64, u64, u64, Adt3, u32, List<Adt4>, List<Adt4>),
}

#[derive(Clone)]
pub enum Adt3 {
    C0(u32, u32, u32, u32, u32, u32, u32, u32),
}

#[derive(Clone)]
pub enum Adt4 {
    C0(Adt5),
    C1(Adt7, u64),
    C2(Adt7, u64),
}

#[derive(Clone)]
pub enum Adt5 {
    C0(u64, u64, u64, u64, u64, Adt3, Adt6, u32, u32, u32, u32, Adt6),
}

#[derive(Clone)]
pub enum Adt6 {
    C0(u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32),
}

#[derive(Clone)]
pub enum Adt7 {
    C0(Adt3, u64, u64),
}

#[derive(Clone)]
pub enum Adt8 {
    C0(Adt3, u32, u64, Bytes, List<Adt7>, List<Adt4>, List<Adt4>),
}

#[derive(Clone)]
pub enum Adt9 {
    C0(u64, u64, u64, u64, u64, Adt3, Adt6, u64, u32, u32),
}

#[derive(Clone)]
pub enum Adt10 {
    C0(Adt7, u64, List<Adt7>, List<Adt4>),
}

#[derive(Clone)]
pub enum Adt11 {
    C0(Adt3, Adt6, u32, u32, u32, u32, List<Adt12>),
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
    C0(Adt6, Adt6, List<Adt12>),
}

#[derive(Clone)]
pub enum Adt15 {
    C0(u64, u64, Adt13, u64, u64),
}

#[derive(Clone)]
pub enum Fn0 {
    F6,
}

impl Fn0 {
    pub fn apply(&self, p0: Adt8, p1: Bytes) -> R<Adt8> {
        match self {
            Fn0::F6 => f6(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn1 {
    F8,
}

impl Fn1 {
    pub fn apply(&self, p0: List<Bytes>, p1: Adt4) -> List<Bytes> {
        match self {
            Fn1::F8 => f8(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn2 {
    F11,
    F64,
}

impl Fn2 {
    pub fn apply(&self, p0: List<Bytes>, p1: Bytes) -> List<Bytes> {
        match self {
            Fn2::F11 => f11(p0, p1),
            Fn2::F64 => f64(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn3 {
    F19(u32, Adt3),
}

impl Fn3 {
    pub fn apply(&self, p0: (Adt9, (List<Adt4>, u64)), p1: Adt7) -> R<(Adt9, (List<Adt4>, u64))> {
        match self {
            Fn3::F19(k0, k1) => f19(*k0, k1.clone(), p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn4 {
    F21((Adt9, (List<Adt4>, u64))),
}

impl Fn4 {
    pub fn apply(&self, p0: (u64, List<Adt4>)) -> R<(u64, List<Adt4>)> {
        match self {
            Fn4::F21(k0) => f21(k0.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn5 {
    F27,
}

impl Fn5 {
    pub fn apply(&self, p0: List<Bytes>) -> Option<List<Bytes>> {
        match self {
            Fn5::F27 => f27(p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn6 {
    F35(u32, u64, u64, Adt3, u64, Bytes),
}

impl Fn6 {
    pub fn apply(&self, p0: (u64, (Adt3, List<Adt4>))) -> R<(u64, (Adt3, List<Adt4>))> {
        match self {
            Fn6::F35(k0, k1, k2, k3, k4, k5) => f35(*k0, *k1, *k2, k3.clone(), *k4, k5.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn7 {
    F44,
}

impl Fn7 {
    pub fn apply(&self, p0: List<Adt4>, p1: Adt4) -> List<Adt4> {
        match self {
            Fn7::F44 => f44(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn8 {
    F46(u32, Bytes, Adt3, Adt8),
}

impl Fn8 {
    pub fn apply(&self, p0: (u64, (List<Adt7>, List<Adt4>))) -> R<(u64, (List<Adt7>, List<Adt4>))> {
        match self {
            Fn8::F46(k0, k1, k2, k3) => f46(*k0, k1.clone(), k2.clone(), k3.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn9 {
    F51(u64),
}

impl Fn9 {
    pub fn apply(&self, p0: Adt6, p1: Adt4) -> Adt6 {
        match self {
            Fn9::F51(k0) => f51(*k0, p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn10 {
    F63,
}

impl Fn10 {
    pub fn apply(&self, p0: (Option<Bytes>, List<Bytes>), p1: Bytes) -> (Option<Bytes>, List<Bytes>) {
        match self {
            Fn10::F63 => f63(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn11 {
    F68,
}

impl Fn11 {
    pub fn apply(&self, p0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
        match self {
            Fn11::F68 => f68(p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn12 {
    F72(u32, Adt3),
}

impl Fn12 {
    pub fn apply(&self, p0: Adt10) -> R<Option<Adt10>> {
        match self {
            Fn12::F72(k0, k1) => f72(*k0, k1.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn13 {
    F75,
}

impl Fn13 {
    pub fn apply(&self, p0: Adt6, p1: Adt12) -> Adt6 {
        match self {
            Fn13::F75 => f75(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn14 {
    F77(Adt3, Adt6),
}

impl Fn14 {
    pub fn apply(&self, p0: (u64, Adt14)) -> R<(u64, Adt14)> {
        match self {
            Fn14::F77(k0, k1) => f77(k0.clone(), k1.clone(), p0),
        }
    }
}

pub fn f0(v0: Adt0, v1: List<Bytes>, v2: u64) -> R<Result<Bytes, Adt1>> {
    let m1 = f1(v0, v1, v2)?;
    match m1 {
        Ok(v3) => {
            Ok(Ok::<Bytes, Adt1>(f2(v3)))
        }
        Err(v4) => {
            Ok(Err::<Bytes, Adt1>(v4))
        }
    }
}

pub fn f1(v0: Adt0, v1: List<Bytes>, v2: u64) -> R<Result<Adt2, Adt1>> {
    let m2 = f3(v0)?;
    match m2 {
        Ok(v3) => {
            Ok(Ok::<Adt2, Adt1>(f4(f5(Fn0::F6, v3, v1)?, v2)?))
        }
        Err(v4) => {
            Ok(Err::<Adt2, Adt1>(v4))
        }
    }
}

pub fn f2(v0: Adt2) -> Bytes {
    let v1: List<Bytes> = f7(Fn1::F8, List::<Bytes>::nil(), match v0.clone() {
        Adt2::C0(_, _, _, _, _, _, h3) => {
            h3
        }
    });
    let v2: Bytes = f9(f10(Fn2::F11, List::<Bytes>::nil(), v1));
    let m8 = {
        let a5 = v2.clone();
        let a6 = 0u64;
        let a7 = match v0 {
            Adt2::C0(_, _, h4, _, _, _, _) => {
                h4
            }
        };
        slice_bytes(a5, a6, a7)
    };
    match m8 {
        None => {
            v2
        }
        Some(v3) => {
            v3
        }
    }
}

pub fn f3(v0: Adt0) -> R<Result<Adt8, Adt1>> {
    let m9 = v0;
    match m9 {
        Adt0::C0 => {
            Ok(Ok::<Adt8, Adt1>(f12(f13(), 0u32)))
        }
        Adt0::C1(v1) => {
            let a11 = {
                let a10 = v1.clone();
                length_bytes(a10)
            };
            let a12 = 32u64;
            if nat_eq(a11, a12) {
                Ok(Ok::<Adt8, Adt1>(f12(f14(v1)?, f15())))
            } else {
                Ok(Err::<Adt8, Adt1>(Adt1::C0({
                    let a13 = v1;
                    length_bytes(a13)
                })))
            }
        }
        Adt0::C2(v2) => {
            Ok(Ok::<Adt8, Adt1>(f16(v2)?))
        }
    }
}

pub fn f4(v0: Adt8, v1: u64) -> R<Adt2> {
    let v2: Adt3 = match v0.clone() {
        Adt8::C0(h14, _, _, _, _, _, _) => {
            h14
        }
    };
    let v3: u32 = match v0.clone() {
        Adt8::C0(_, h15, _, _, _, _, _) => {
            h15
        }
    };
    let v4: Bytes = match v0.clone() {
        Adt8::C0(_, _, _, h16, _, _, _) => {
            h16
        }
    };
    let v5: (Adt9, List<Adt4>) = f17(v2.clone(), v3, v4.clone(), 0u64, {
        let a17 = v4.clone();
        length_bytes(a17)
    }, match v0.clone() {
        Adt8::C0(_, _, h18, _, _, _, _) => {
            h18
        }
    }, match v0.clone() {
        Adt8::C0(_, _, _, _, _, h19, _) => {
            h19
        }
    })?;
    let v6: (Adt9, (List<Adt4>, u64)) = f18(Fn3::F19(v3, v2.clone()), ({
        let (h20, _) = v5;
        h20
    }, ({
        let (_, h21) = v5;
        h21
    }, {
        let a23 = match v0.clone() {
            Adt8::C0(_, _, _, _, h22, _, _) => {
                h22
            }
        };
        length_list(a23)
    })), match v0.clone() {
        Adt8::C0(_, _, _, _, h24, _, _) => {
            h24
        }
    })?;
    let v7: (u64, List<Adt4>) = f20(Fn4::F21(v6.clone()), {
        let a27 = {
            let a25 = v1;
            let a26 = 63u64;
            nat_add(a25, a26)?
        };
        let a28 = 64u64;
        let a29 = 0u64;
        nat_quot(a27, a28, a29)
    }, (0u64, {
        let (h30, _) = {
            let (_, h31) = v6;
            h31
        };
        h30
    }))?;
    Ok(Adt2::C0(f22(v3), {
        let a36 = {
            let a33 = match v0.clone() {
                Adt8::C0(_, _, h32, _, _, _, _) => {
                    h32
                }
            };
            let a34 = 1024u64;
            nat_mul(a33, a34)?
        };
        let a37 = {
            let a35 = v4;
            length_bytes(a35)
        };
        nat_add(a36, a37)?
    }, v1, v2, v3, match v0 {
        Adt8::C0(_, _, _, _, _, _, h38) => {
            h38
        }
    }, f23({
        let (_, h39) = v7;
        h39
    })))
}

pub fn f5(v0: Fn0, p43: Adt8, p44: List<Bytes>) -> R<Adt8> {
    let m42 = (p43, p44);
    {
        let mut m42 = m42;
        loop {
            let step = {
                let (v1, v2) = m42;
                let m40 = v2;
                match m40.uncons() {
                    None => {
                        Err::<(Adt8, List<Bytes>), R<Adt8>>(Ok(v1))
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt8, List<Bytes>), R<Adt8>>(({
                            let c41: Fn0 = v0.clone();
                            c41.apply(v1, v3)?
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m42 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f6(v0: Adt8, v1: Bytes) -> R<Adt8> {
    f24(v0, v1)
}

pub fn f7(v0: Fn1, p48: List<Bytes>, p49: List<Adt4>) -> List<Bytes> {
    let m47 = (p48, p49);
    {
        let mut m47 = m47;
        loop {
            let step = {
                let (v1, v2) = m47;
                let m45 = v2;
                match m45.uncons() {
                    None => {
                        Err::<(List<Bytes>, List<Adt4>), List<Bytes>>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(List<Bytes>, List<Adt4>), List<Bytes>>(({
                            let c46: Fn1 = v0.clone();
                            c46.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m47 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f8(v0: List<Bytes>, v1: Adt4) -> List<Bytes> {
    let m50 = v1;
    match m50 {
        Adt4::C0(v2) => {
            let a52 = match v2.clone() {
                Adt5::C0(h51, _, _, _, _, _, _, _, _, _, _, _) => {
                    h51
                }
            };
            let a53 = 2u64;
            if nat_eq(a52, a53) {
                List::cons(f25(match v2 {
                    Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h54) => {
                        h54
                    }
                }), v0)
            } else {
                v0
            }
        }
        Adt4::C1(_, _) => {
            v0
        }
        Adt4::C2(_, _) => {
            v0
        }
    }
}

pub fn f9(v0: List<Bytes>) -> Bytes {
    let m56 = {
        let (h55, _) = f26(Fn5::F27, 64u64, v0);
        h55
    };
    match m56.uncons() {
        None => {
            Bytes::lit(&[])
        }
        Some((v1, _)) => {
            v1
        }
    }
}

pub fn f10(v0: Fn2, p60: List<Bytes>, p61: List<Bytes>) -> List<Bytes> {
    let m59 = (p60, p61);
    {
        let mut m59 = m59;
        loop {
            let step = {
                let (v1, v2) = m59;
                let m57 = v2;
                match m57.uncons() {
                    None => {
                        Err::<(List<Bytes>, List<Bytes>), List<Bytes>>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(List<Bytes>, List<Bytes>), List<Bytes>>(({
                            let c58: Fn2 = v0.clone();
                            c58.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m59 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f11(v0: List<Bytes>, v1: Bytes) -> List<Bytes> {
    List::cons(v1, v0)
}

pub fn f12(v0: Adt3, v1: u32) -> Adt8 {
    Adt8::C0(v0, v1, 0u64, Bytes::lit(&[]), List::<Adt7>::nil(), List::<Adt4>::nil(), List::<Adt4>::nil())
}

pub fn f13() -> Adt3 {
    Adt3::C0(1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, 1359893119u32, 2600822924u32, 528734635u32, 1541459225u32)
}

pub fn f14(v0: Bytes) -> R<Adt3> {
    Ok(f28(f29(v0.clone(), {
        let a62 = v0;
        length_bytes(a62)
    }, 0u64)?))
}

pub fn f15() -> u32 {
    16u32
}

pub fn f16(v0: Bytes) -> R<Adt8> {
    let v1: Adt2 = f30(v0)?;
    Ok(Adt8::C0(f28(f31(v1.clone(), 0u64)), f32(), 0u64, Bytes::lit(&[]), List::<Adt7>::nil(), List::<Adt4>::nil(), match v1 {
        Adt2::C0(_, _, _, _, _, _, h63) => {
            h63
        }
    }))
}

pub fn f17(v0: Adt3, v1: u32, v2: Bytes, v3: u64, v4: u64, v5: u64, v6: List<Adt4>) -> R<(Adt9, List<Adt4>)> {
    let v7: u64 = {
        let a64 = v3;
        let a65 = v4;
        nat_add(a64, a65)?
    };
    let v8: u64 = f33(v4)?;
    let v9: (u64, (Adt3, List<Adt4>)) = f34(Fn6::F35(v1, v3, v5, v0.clone(), v7, v2.clone()), {
        let a66 = v8;
        let a67 = 64u64;
        let a68 = 0u64;
        nat_quot(a66, a67, a68)
    }, (0u64, (v0.clone(), v6)))?;
    Ok((f36(v0, v1, v2, v7, v3, v5, v8, {
        let (h69, _) = {
            let (_, h70) = v9.clone();
            h70
        };
        h69
    }, {
        let a71 = v4;
        let a72 = v8;
        nat_sub(a71, a72)
    }, {
        let a73 = f37();
        let a74 = f38(v8);
        fixed_u32::bit_or(a73, a74)
    })?, {
        let (_, h75) = {
            let (_, h76) = v9;
            h76
        };
        h75
    }))
}

pub fn f18(v0: Fn3, p80: (Adt9, (List<Adt4>, u64)), p81: List<Adt7>) -> R<(Adt9, (List<Adt4>, u64))> {
    let m79 = (p80, p81);
    {
        let mut m79 = m79;
        loop {
            let step = {
                let (v1, v2) = m79;
                let m77 = v2;
                match m77.uncons() {
                    None => {
                        Err::<((Adt9, (List<Adt4>, u64)), List<Adt7>), R<(Adt9, (List<Adt4>, u64))>>(Ok(v1))
                    }
                    Some((v3, v4)) => {
                        Ok::<((Adt9, (List<Adt4>, u64)), List<Adt7>), R<(Adt9, (List<Adt4>, u64))>>(({
                            let c78: Fn3 = v0.clone();
                            c78.apply(v1, v3)?
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

pub fn f19(v0: u32, v1: Adt3, v2: (Adt9, (List<Adt4>, u64)), v3: Adt7) -> R<(Adt9, (List<Adt4>, u64))> {
    let v4: Adt5 = f39({
        let (h82, _) = v2.clone();
        h82
    })?;
    Ok((f40(v3.clone(), Adt7::C0(f28(match v4.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h83) => {
            h83
        }
    }), {
        let m86 = {
            let (h85, _) = v2.clone();
            h85
        };
        match m86 {
            Adt9::C0(_, h84, _, _, _, _, _, _, _, _) => {
                h84
            }
        }
    }, {
        let m89 = {
            let (h88, _) = v2.clone();
            h88
        };
        match m89 {
            Adt9::C0(_, _, h87, _, _, _, _, _, _, _) => {
                h87
            }
        }
    }), v1, v0)?, (List::cons(Adt4::C2(v3, {
        let (_, h90) = {
            let (_, h91) = v2.clone();
            h91
        };
        h90
    }), List::cons(Adt4::C0(v4), {
        let (h92, _) = {
            let (_, h93) = v2.clone();
            h93
        };
        h92
    })), {
        let a96 = {
            let (_, h94) = {
                let (_, h95) = v2;
                h95
            };
            h94
        };
        let a97 = 1u64;
        nat_sub(a96, a97)
    })))
}

pub fn f20(v0: Fn4, p101: u64, p102: (u64, List<Adt4>)) -> R<(u64, List<Adt4>)> {
    let m100 = (p101, p102);
    {
        let mut m100 = m100;
        loop {
            let step = {
                let (v1, v2) = m100;
                let m98 = v1;
                if m98 == 0 {
                    Err::<(u64, (u64, List<Adt4>)), R<(u64, List<Adt4>)>>(Ok(v2))
                } else {
                    let v3 = m98 - 1;
                    Ok::<(u64, (u64, List<Adt4>)), R<(u64, List<Adt4>)>>((v3, {
                        let c99: Fn4 = v0.clone();
                        c99.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m100 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f21(v0: (Adt9, (List<Adt4>, u64)), v1: (u64, List<Adt4>)) -> R<(u64, List<Adt4>)> {
    Ok(({
        let a104 = {
            let (h103, _) = v1.clone();
            h103
        };
        let a105 = 1u64;
        nat_add(a104, a105)?
    }, List::cons(Adt4::C0(f39(f41({
        let (h106, _) = v0;
        h106
    }, {
        let (h107, _) = v1;
        h107
    }))?), {
        let (_, h108) = v1;
        h108
    })))
}

pub fn f22(v0: u32) -> u64 {
    let a109 = v0;
    let a110 = f15();
    if equal(a109, a110) {
        1u64
    } else {
        let a111 = v0;
        let a112 = f32();
        if equal(a111, a112) {
            2u64
        } else {
            let a113 = v0;
            let a114 = f42();
            if equal(a113, a114) {
                3u64
            } else {
                0u64
            }
        }
    }
}

pub fn f23(v0: List<Adt4>) -> List<Adt4> {
    f43(Fn7::F44, List::<Adt4>::nil(), v0)
}

pub fn f24(v0: Adt8, v1: Bytes) -> R<Adt8> {
    let v2: Bytes = {
        let a116 = match v0.clone() {
            Adt8::C0(_, _, _, h115, _, _, _) => {
                h115
            }
        };
        let a117 = v1;
        append_bytes(a116, a117)
    };
    let v3: u64 = {
        let a118 = v2.clone();
        length_bytes(a118)
    };
    let v4: u64 = {
        let a119 = v3;
        let a120 = 0u64;
        if nat_eq(a119, a120) {
            0u64
        } else {
            let a123 = {
                let a121 = v3;
                let a122 = 1u64;
                nat_sub(a121, a122)
            };
            let a124 = 1024u64;
            let a125 = 0u64;
            nat_quot(a123, a124, a125)
        }
    };
    let v5: u32 = match v0.clone() {
        Adt8::C0(_, h126, _, _, _, _, _) => {
            h126
        }
    };
    let v6: Adt3 = match v0.clone() {
        Adt8::C0(h127, _, _, _, _, _, _) => {
            h127
        }
    };
    let v7: (u64, (List<Adt7>, List<Adt4>)) = f45(Fn8::F46(v5, v2.clone(), v6.clone(), v0.clone()), v4, (0u64, (match v0.clone() {
        Adt8::C0(_, _, _, _, h128, _, _) => {
            h128
        }
    }, match v0.clone() {
        Adt8::C0(_, _, _, _, _, h129, _) => {
            h129
        }
    })))?;
    let v8: u64 = {
        let a130 = v4;
        let a131 = 1024u64;
        nat_mul(a130, a131)?
    };
    Ok(Adt8::C0(v6, v5, {
        let a133 = match v0.clone() {
            Adt8::C0(_, _, h132, _, _, _, _) => {
                h132
            }
        };
        let a134 = v4;
        nat_add(a133, a134)?
    }, {
        let m140 = {
            let a137 = v2;
            let a138 = v8;
            let a139 = {
                let a135 = v3;
                let a136 = v8;
                nat_sub(a135, a136)
            };
            slice_bytes(a137, a138, a139)
        };
        match m140 {
            None => {
                Bytes::lit(&[])
            }
            Some(v9) => {
                v9
            }
        }
    }, {
        let (h141, _) = {
            let (_, h142) = v7.clone();
            h142
        };
        h141
    }, {
        let (_, h143) = {
            let (_, h144) = v7;
            h144
        };
        h143
    }, match v0 {
        Adt8::C0(_, _, _, _, _, _, h145) => {
            h145
        }
    }))
}

pub fn f25(v0: Adt6) -> Bytes {
    let a190 = f47(match v0.clone() {
        Adt6::C0(h146, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h146
        }
    });
    let a191 = {
        let a188 = f47(match v0.clone() {
            Adt6::C0(_, h147, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                h147
            }
        });
        let a189 = {
            let a186 = f47(match v0.clone() {
                Adt6::C0(_, _, h148, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                    h148
                }
            });
            let a187 = {
                let a184 = f47(match v0.clone() {
                    Adt6::C0(_, _, _, h149, _, _, _, _, _, _, _, _, _, _, _, _) => {
                        h149
                    }
                });
                let a185 = {
                    let a182 = f47(match v0.clone() {
                        Adt6::C0(_, _, _, _, h150, _, _, _, _, _, _, _, _, _, _, _) => {
                            h150
                        }
                    });
                    let a183 = {
                        let a180 = f47(match v0.clone() {
                            Adt6::C0(_, _, _, _, _, h151, _, _, _, _, _, _, _, _, _, _) => {
                                h151
                            }
                        });
                        let a181 = {
                            let a178 = f47(match v0.clone() {
                                Adt6::C0(_, _, _, _, _, _, h152, _, _, _, _, _, _, _, _, _) => {
                                    h152
                                }
                            });
                            let a179 = {
                                let a176 = f47(match v0.clone() {
                                    Adt6::C0(_, _, _, _, _, _, _, h153, _, _, _, _, _, _, _, _) => {
                                        h153
                                    }
                                });
                                let a177 = {
                                    let a174 = f47(match v0.clone() {
                                        Adt6::C0(_, _, _, _, _, _, _, _, h154, _, _, _, _, _, _, _) => {
                                            h154
                                        }
                                    });
                                    let a175 = {
                                        let a172 = f47(match v0.clone() {
                                            Adt6::C0(_, _, _, _, _, _, _, _, _, h155, _, _, _, _, _, _) => {
                                                h155
                                            }
                                        });
                                        let a173 = {
                                            let a170 = f47(match v0.clone() {
                                                Adt6::C0(_, _, _, _, _, _, _, _, _, _, h156, _, _, _, _, _) => {
                                                    h156
                                                }
                                            });
                                            let a171 = {
                                                let a168 = f47(match v0.clone() {
                                                    Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h157, _, _, _, _) => {
                                                        h157
                                                    }
                                                });
                                                let a169 = {
                                                    let a166 = f47(match v0.clone() {
                                                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h158, _, _, _) => {
                                                            h158
                                                        }
                                                    });
                                                    let a167 = {
                                                        let a164 = f47(match v0.clone() {
                                                            Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h159, _, _) => {
                                                                h159
                                                            }
                                                        });
                                                        let a165 = {
                                                            let a162 = f47(match v0.clone() {
                                                                Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h160, _) => {
                                                                    h160
                                                                }
                                                            });
                                                            let a163 = f47(match v0 {
                                                                Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h161) => {
                                                                    h161
                                                                }
                                                            });
                                                            append_bytes(a162, a163)
                                                        };
                                                        append_bytes(a164, a165)
                                                    };
                                                    append_bytes(a166, a167)
                                                };
                                                append_bytes(a168, a169)
                                            };
                                            append_bytes(a170, a171)
                                        };
                                        append_bytes(a172, a173)
                                    };
                                    append_bytes(a174, a175)
                                };
                                append_bytes(a176, a177)
                            };
                            append_bytes(a178, a179)
                        };
                        append_bytes(a180, a181)
                    };
                    append_bytes(a182, a183)
                };
                append_bytes(a184, a185)
            };
            append_bytes(a186, a187)
        };
        append_bytes(a188, a189)
    };
    append_bytes(a190, a191)
}

pub fn f26(v0: Fn5, p196: u64, p197: List<Bytes>) -> (List<Bytes>, bool) {
    let m195 = (p196, p197);
    {
        let mut m195 = m195;
        loop {
            let step = {
                let (v1, v2) = m195;
                let m192 = v1;
                if m192 == 0 {
                    Err::<(u64, List<Bytes>), (List<Bytes>, bool)>((v2, false))
                } else {
                    let v3 = m192 - 1;
                    let m194 = {
                        let c193: Fn5 = v0.clone();
                        c193.apply(v2.clone())
                    };
                    match m194 {
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
                Ok(next) => { m195 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f27(v0: List<Bytes>) -> Option<List<Bytes>> {
    let a199 = {
        let a198 = v0.clone();
        length_list(a198)
    };
    let a200 = 1u64;
    if nat_le(a199, a200) {
        None::<List<Bytes>>
    } else {
        Some(f48(v0))
    }
}

pub fn f28(v0: Adt6) -> Adt3 {
    Adt3::C0(match v0.clone() {
        Adt6::C0(h201, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h201
        }
    }, match v0.clone() {
        Adt6::C0(_, h202, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h202
        }
    }, match v0.clone() {
        Adt6::C0(_, _, h203, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h203
        }
    }, match v0.clone() {
        Adt6::C0(_, _, _, h204, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h204
        }
    }, match v0.clone() {
        Adt6::C0(_, _, _, _, h205, _, _, _, _, _, _, _, _, _, _, _) => {
            h205
        }
    }, match v0.clone() {
        Adt6::C0(_, _, _, _, _, h206, _, _, _, _, _, _, _, _, _, _) => {
            h206
        }
    }, match v0.clone() {
        Adt6::C0(_, _, _, _, _, _, h207, _, _, _, _, _, _, _, _, _) => {
            h207
        }
    }, match v0 {
        Adt6::C0(_, _, _, _, _, _, _, h208, _, _, _, _, _, _, _, _) => {
            h208
        }
    })
}

pub fn f29(v0: Bytes, v1: u64, v2: u64) -> R<Adt6> {
    Ok(Adt6::C0(f49(v0.clone(), v1, {
        let a209 = v2;
        let a210 = 0u64;
        nat_add(a209, a210)?
    })?, f49(v0.clone(), v1, {
        let a211 = v2;
        let a212 = 4u64;
        nat_add(a211, a212)?
    })?, f49(v0.clone(), v1, {
        let a213 = v2;
        let a214 = 8u64;
        nat_add(a213, a214)?
    })?, f49(v0.clone(), v1, {
        let a215 = v2;
        let a216 = 12u64;
        nat_add(a215, a216)?
    })?, f49(v0.clone(), v1, {
        let a217 = v2;
        let a218 = 16u64;
        nat_add(a217, a218)?
    })?, f49(v0.clone(), v1, {
        let a219 = v2;
        let a220 = 20u64;
        nat_add(a219, a220)?
    })?, f49(v0.clone(), v1, {
        let a221 = v2;
        let a222 = 24u64;
        nat_add(a221, a222)?
    })?, f49(v0.clone(), v1, {
        let a223 = v2;
        let a224 = 28u64;
        nat_add(a223, a224)?
    })?, f49(v0.clone(), v1, {
        let a225 = v2;
        let a226 = 32u64;
        nat_add(a225, a226)?
    })?, f49(v0.clone(), v1, {
        let a227 = v2;
        let a228 = 36u64;
        nat_add(a227, a228)?
    })?, f49(v0.clone(), v1, {
        let a229 = v2;
        let a230 = 40u64;
        nat_add(a229, a230)?
    })?, f49(v0.clone(), v1, {
        let a231 = v2;
        let a232 = 44u64;
        nat_add(a231, a232)?
    })?, f49(v0.clone(), v1, {
        let a233 = v2;
        let a234 = 48u64;
        nat_add(a233, a234)?
    })?, f49(v0.clone(), v1, {
        let a235 = v2;
        let a236 = 52u64;
        nat_add(a235, a236)?
    })?, f49(v0.clone(), v1, {
        let a237 = v2;
        let a238 = 56u64;
        nat_add(a237, a238)?
    })?, f49(v0, v1, {
        let a239 = v2;
        let a240 = 60u64;
        nat_add(a239, a240)?
    })?))
}

pub fn f30(v0: Bytes) -> R<Adt2> {
    f4(f24(f12(f13(), f42()), v0)?, 32u64)
}

pub fn f31(v0: Adt2, v1: u64) -> Adt6 {
    f50(Fn9::F51(v1), Adt6::C0(0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32), match v0 {
        Adt2::C0(_, _, _, _, _, _, h241) => {
            h241
        }
    })
}

pub fn f32() -> u32 {
    64u32
}

pub fn f33(v0: u64) -> R<u64> {
    let a242 = v0;
    let a243 = 0u64;
    if nat_eq(a242, a243) {
        Ok(0u64)
    } else {
        let a249 = {
            let a246 = {
                let a244 = v0;
                let a245 = 1u64;
                nat_sub(a244, a245)
            };
            let a247 = 64u64;
            let a248 = 0u64;
            nat_quot(a246, a247, a248)
        };
        let a250 = 64u64;
        nat_mul(a249, a250)
    }
}

pub fn f34(v0: Fn6, p254: u64, p255: (u64, (Adt3, List<Adt4>))) -> R<(u64, (Adt3, List<Adt4>))> {
    let m253 = (p254, p255);
    {
        let mut m253 = m253;
        loop {
            let step = {
                let (v1, v2) = m253;
                let m251 = v1;
                if m251 == 0 {
                    Err::<(u64, (u64, (Adt3, List<Adt4>))), R<(u64, (Adt3, List<Adt4>))>>(Ok(v2))
                } else {
                    let v3 = m251 - 1;
                    Ok::<(u64, (u64, (Adt3, List<Adt4>))), R<(u64, (Adt3, List<Adt4>))>>((v3, {
                        let c252: Fn6 = v0.clone();
                        c252.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m253 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f35(v0: u32, v1: u64, v2: u64, v3: Adt3, v4: u64, v5: Bytes, v6: (u64, (Adt3, List<Adt4>))) -> R<(u64, (Adt3, List<Adt4>))> {
    let v7: u64 = {
        let (h256, _) = v6.clone();
        h256
    };
    let v8: Adt5 = f39(f36(v3, v0, v5, v4, v1, v2, v7, {
        let (h257, _) = {
            let (_, h258) = v6.clone();
            h258
        };
        h257
    }, 64u64, f38(v7))?)?;
    Ok(({
        let a259 = v7;
        let a260 = 64u64;
        nat_add(a259, a260)?
    }, (f28(match v8.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h261) => {
            h261
        }
    }), List::cons(Adt4::C0(v8), {
        let (_, h262) = {
            let (_, h263) = v6;
            h263
        };
        h262
    }))))
}

pub fn f36(_: Adt3, v1: u32, v2: Bytes, v3: u64, v4: u64, v5: u64, v6: u64, v7: Adt3, v8: u64, v9: u32) -> R<Adt9> {
    Ok(Adt9::C0(0u64, v5, 1u64, {
        let a264 = v6;
        let a265 = 64u64;
        let a266 = 0u64;
        nat_quot(a264, a265, a266)
    }, {
        let a269 = {
            let a267 = v5;
            let a268 = 1024u64;
            nat_mul(a267, a268)?
        };
        let a270 = v6;
        nat_add(a269, a270)?
    }, v7, f29(v2, v3, {
        let a271 = v4;
        let a272 = v6;
        nat_add(a271, a272)?
    })?, v5, f52(v8, 7u64), {
        let a273 = v1;
        let a274 = v9;
        fixed_u32::bit_or(a273, a274)
    }))
}

pub fn f37() -> u32 {
    2u32
}

pub fn f38(v0: u64) -> u32 {
    let a275 = v0;
    let a276 = 0u64;
    if nat_eq(a275, a276) {
        f53()
    } else {
        0u32
    }
}

pub fn f39(v0: Adt9) -> R<Adt5> {
    let v1: u32 = f54(match v0.clone() {
        Adt9::C0(_, _, _, _, _, _, _, h277, _, _) => {
            h277
        }
    });
    let v2: u32 = f55(match v0.clone() {
        Adt9::C0(_, _, _, _, _, _, _, h278, _, _) => {
            h278
        }
    });
    Ok(Adt5::C0(match v0.clone() {
        Adt9::C0(h279, _, _, _, _, _, _, _, _, _) => {
            h279
        }
    }, match v0.clone() {
        Adt9::C0(_, h280, _, _, _, _, _, _, _, _) => {
            h280
        }
    }, match v0.clone() {
        Adt9::C0(_, _, h281, _, _, _, _, _, _, _) => {
            h281
        }
    }, match v0.clone() {
        Adt9::C0(_, _, _, h282, _, _, _, _, _, _) => {
            h282
        }
    }, match v0.clone() {
        Adt9::C0(_, _, _, _, h283, _, _, _, _, _) => {
            h283
        }
    }, match v0.clone() {
        Adt9::C0(_, _, _, _, _, h284, _, _, _, _) => {
            h284
        }
    }, match v0.clone() {
        Adt9::C0(_, _, _, _, _, _, h285, _, _, _) => {
            h285
        }
    }, v1, v2, match v0.clone() {
        Adt9::C0(_, _, _, _, _, _, _, _, h286, _) => {
            h286
        }
    }, match v0.clone() {
        Adt9::C0(_, _, _, _, _, _, _, _, _, h287) => {
            h287
        }
    }, f56(match v0.clone() {
        Adt9::C0(_, _, _, _, _, h288, _, _, _, _) => {
            h288
        }
    }, match v0.clone() {
        Adt9::C0(_, _, _, _, _, _, h289, _, _, _) => {
            h289
        }
    }, v1, v2, match v0.clone() {
        Adt9::C0(_, _, _, _, _, _, _, _, h290, _) => {
            h290
        }
    }, match v0 {
        Adt9::C0(_, _, _, _, _, _, _, _, _, h291) => {
            h291
        }
    })?))
}

pub fn f40(v0: Adt7, v1: Adt7, v2: Adt3, v3: u32) -> R<Adt9> {
    Ok(Adt9::C0(1u64, match v0.clone() {
        Adt7::C0(_, h292, _) => {
            h292
        }
    }, {
        let a295 = match v0.clone() {
            Adt7::C0(_, _, h293) => {
                h293
            }
        };
        let a296 = match v1.clone() {
            Adt7::C0(_, _, h294) => {
                h294
            }
        };
        nat_add(a295, a296)?
    }, 0u64, 0u64, v2, Adt6::C0({
        let m299 = match v0.clone() {
            Adt7::C0(h298, _, _) => {
                h298
            }
        };
        match m299 {
            Adt3::C0(h297, _, _, _, _, _, _, _) => {
                h297
            }
        }
    }, {
        let m302 = match v0.clone() {
            Adt7::C0(h301, _, _) => {
                h301
            }
        };
        match m302 {
            Adt3::C0(_, h300, _, _, _, _, _, _) => {
                h300
            }
        }
    }, {
        let m305 = match v0.clone() {
            Adt7::C0(h304, _, _) => {
                h304
            }
        };
        match m305 {
            Adt3::C0(_, _, h303, _, _, _, _, _) => {
                h303
            }
        }
    }, {
        let m308 = match v0.clone() {
            Adt7::C0(h307, _, _) => {
                h307
            }
        };
        match m308 {
            Adt3::C0(_, _, _, h306, _, _, _, _) => {
                h306
            }
        }
    }, {
        let m311 = match v0.clone() {
            Adt7::C0(h310, _, _) => {
                h310
            }
        };
        match m311 {
            Adt3::C0(_, _, _, _, h309, _, _, _) => {
                h309
            }
        }
    }, {
        let m314 = match v0.clone() {
            Adt7::C0(h313, _, _) => {
                h313
            }
        };
        match m314 {
            Adt3::C0(_, _, _, _, _, h312, _, _) => {
                h312
            }
        }
    }, {
        let m317 = match v0.clone() {
            Adt7::C0(h316, _, _) => {
                h316
            }
        };
        match m317 {
            Adt3::C0(_, _, _, _, _, _, h315, _) => {
                h315
            }
        }
    }, {
        let m320 = match v0 {
            Adt7::C0(h319, _, _) => {
                h319
            }
        };
        match m320 {
            Adt3::C0(_, _, _, _, _, _, _, h318) => {
                h318
            }
        }
    }, {
        let m323 = match v1.clone() {
            Adt7::C0(h322, _, _) => {
                h322
            }
        };
        match m323 {
            Adt3::C0(h321, _, _, _, _, _, _, _) => {
                h321
            }
        }
    }, {
        let m326 = match v1.clone() {
            Adt7::C0(h325, _, _) => {
                h325
            }
        };
        match m326 {
            Adt3::C0(_, h324, _, _, _, _, _, _) => {
                h324
            }
        }
    }, {
        let m329 = match v1.clone() {
            Adt7::C0(h328, _, _) => {
                h328
            }
        };
        match m329 {
            Adt3::C0(_, _, h327, _, _, _, _, _) => {
                h327
            }
        }
    }, {
        let m332 = match v1.clone() {
            Adt7::C0(h331, _, _) => {
                h331
            }
        };
        match m332 {
            Adt3::C0(_, _, _, h330, _, _, _, _) => {
                h330
            }
        }
    }, {
        let m335 = match v1.clone() {
            Adt7::C0(h334, _, _) => {
                h334
            }
        };
        match m335 {
            Adt3::C0(_, _, _, _, h333, _, _, _) => {
                h333
            }
        }
    }, {
        let m338 = match v1.clone() {
            Adt7::C0(h337, _, _) => {
                h337
            }
        };
        match m338 {
            Adt3::C0(_, _, _, _, _, h336, _, _) => {
                h336
            }
        }
    }, {
        let m341 = match v1.clone() {
            Adt7::C0(h340, _, _) => {
                h340
            }
        };
        match m341 {
            Adt3::C0(_, _, _, _, _, _, h339, _) => {
                h339
            }
        }
    }, {
        let m344 = match v1 {
            Adt7::C0(h343, _, _) => {
                h343
            }
        };
        match m344 {
            Adt3::C0(_, _, _, _, _, _, _, h342) => {
                h342
            }
        }
    }), 0u64, 64u32, {
        let a345 = v3;
        let a346 = f57();
        fixed_u32::bit_or(a345, a346)
    }))
}

pub fn f41(v0: Adt9, v1: u64) -> Adt9 {
    Adt9::C0(2u64, match v0.clone() {
        Adt9::C0(_, h347, _, _, _, _, _, _, _, _) => {
            h347
        }
    }, match v0.clone() {
        Adt9::C0(_, _, h348, _, _, _, _, _, _, _) => {
            h348
        }
    }, v1, match v0.clone() {
        Adt9::C0(_, _, _, _, h349, _, _, _, _, _) => {
            h349
        }
    }, match v0.clone() {
        Adt9::C0(_, _, _, _, _, h350, _, _, _, _) => {
            h350
        }
    }, match v0.clone() {
        Adt9::C0(_, _, _, _, _, _, h351, _, _, _) => {
            h351
        }
    }, v1, match v0.clone() {
        Adt9::C0(_, _, _, _, _, _, _, _, h352, _) => {
            h352
        }
    }, {
        let a354 = match v0 {
            Adt9::C0(_, _, _, _, _, _, _, _, _, h353) => {
                h353
            }
        };
        let a355 = f58();
        fixed_u32::bit_or(a354, a355)
    })
}

pub fn f42() -> u32 {
    32u32
}

pub fn f43(v0: Fn7, p359: List<Adt4>, p360: List<Adt4>) -> List<Adt4> {
    let m358 = (p359, p360);
    {
        let mut m358 = m358;
        loop {
            let step = {
                let (v1, v2) = m358;
                let m356 = v2;
                match m356.uncons() {
                    None => {
                        Err::<(List<Adt4>, List<Adt4>), List<Adt4>>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(List<Adt4>, List<Adt4>), List<Adt4>>(({
                            let c357: Fn7 = v0.clone();
                            c357.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m358 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f44(v0: List<Adt4>, v1: Adt4) -> List<Adt4> {
    List::cons(v1, v0)
}

pub fn f45(v0: Fn8, p364: u64, p365: (u64, (List<Adt7>, List<Adt4>))) -> R<(u64, (List<Adt7>, List<Adt4>))> {
    let m363 = (p364, p365);
    {
        let mut m363 = m363;
        loop {
            let step = {
                let (v1, v2) = m363;
                let m361 = v1;
                if m361 == 0 {
                    Err::<(u64, (u64, (List<Adt7>, List<Adt4>))), R<(u64, (List<Adt7>, List<Adt4>))>>(Ok(v2))
                } else {
                    let v3 = m361 - 1;
                    Ok::<(u64, (u64, (List<Adt7>, List<Adt4>))), R<(u64, (List<Adt7>, List<Adt4>))>>((v3, {
                        let c362: Fn8 = v0.clone();
                        c362.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m363 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f46(v0: u32, v1: Bytes, v2: Adt3, v3: Adt8, v4: (u64, (List<Adt7>, List<Adt4>))) -> R<(u64, (List<Adt7>, List<Adt4>))> {
    let v5: u64 = {
        let a368 = match v3 {
            Adt8::C0(_, _, h366, _, _, _, _) => {
                h366
            }
        };
        let a369 = {
            let (h367, _) = v4.clone();
            h367
        };
        nat_add(a368, a369)?
    };
    let v6: (Adt9, List<Adt4>) = f17(v2.clone(), v0, v1, {
        let a371 = {
            let (h370, _) = v4.clone();
            h370
        };
        let a372 = 1024u64;
        nat_mul(a371, a372)?
    }, 1024u64, v5, {
        let (_, h373) = {
            let (_, h374) = v4.clone();
            h374
        };
        h373
    })?;
    let v7: Adt5 = f39({
        let (h375, _) = v6;
        h375
    })?;
    Ok(({
        let a377 = {
            let (h376, _) = v4;
            h376
        };
        let a378 = 1u64;
        nat_add(a377, a378)?
    }, f59(v2, v0, Adt10::C0(Adt7::C0(f28(match v7.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h379) => {
            h379
        }
    }), v5, 1u64), {
        let a380 = v5;
        let a381 = 1u64;
        nat_add(a380, a381)?
    }, {
        let (h382, _) = {
            let (_, h383) = v4;
            h383
        };
        h382
    }, List::cons(Adt4::C0(v7), {
        let (_, h384) = v6;
        h384
    })))?))
}

pub fn f47(v0: u32) -> Bytes {
    let a395 = f60({
        let a385 = v0;
        let a386 = 255u32;
        fixed_u32::bit_and(a385, a386)
    });
    let a396 = {
        let a393 = f60({
            let a387 = f61(v0, 8u32);
            let a388 = 255u32;
            fixed_u32::bit_and(a387, a388)
        });
        let a394 = {
            let a391 = f60({
                let a389 = f61(v0, 16u32);
                let a390 = 255u32;
                fixed_u32::bit_and(a389, a390)
            });
            let a392 = f60(f61(v0, 24u32));
            append_bytes(a391, a392)
        };
        append_bytes(a393, a394)
    };
    append_bytes(a395, a396)
}

pub fn f48(v0: List<Bytes>) -> List<Bytes> {
    let v1: (Option<Bytes>, List<Bytes>) = f62(Fn10::F63, (None::<Bytes>, List::<Bytes>::nil()), v0);
    let v3: List<Bytes> = {
        let m398 = {
            let (h397, _) = v1;
            h397
        };
        match m398 {
            None => {
                let (_, h400) = v1;
                h400
            }
            Some(v2) => {
                List::cons(v2, {
                    let (_, h399) = v1;
                    h399
                })
            }
        }
    };
    f10(Fn2::F64, List::<Bytes>::nil(), v3)
}

pub fn f49(v0: Bytes, v1: u64, v2: u64) -> R<u32> {
    Ok(f65(f66(v0.clone(), v1, v2), f66(v0.clone(), v1, {
        let a401 = v2;
        let a402 = 1u64;
        nat_add(a401, a402)?
    }), f66(v0.clone(), v1, {
        let a403 = v2;
        let a404 = 2u64;
        nat_add(a403, a404)?
    }), f66(v0, v1, {
        let a405 = v2;
        let a406 = 3u64;
        nat_add(a405, a406)?
    })))
}

pub fn f50(v0: Fn9, p410: Adt6, p411: List<Adt4>) -> Adt6 {
    let m409 = (p410, p411);
    {
        let mut m409 = m409;
        loop {
            let step = {
                let (v1, v2) = m409;
                let m407 = v2;
                match m407.uncons() {
                    None => {
                        Err::<(Adt6, List<Adt4>), Adt6>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt6, List<Adt4>), Adt6>(({
                            let c408: Fn9 = v0.clone();
                            c408.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m409 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f51(v0: u64, v1: Adt6, v2: Adt4) -> Adt6 {
    let m412 = v2;
    match m412 {
        Adt4::C0(v3) => {
            let a414 = match v3.clone() {
                Adt5::C0(h413, _, _, _, _, _, _, _, _, _, _, _) => {
                    h413
                }
            };
            let a415 = 2u64;
            if nat_eq(a414, a415) {
                let a417 = match v3.clone() {
                    Adt5::C0(_, _, _, h416, _, _, _, _, _, _, _, _) => {
                        h416
                    }
                };
                let a418 = v0;
                if nat_eq(a417, a418) {
                    match v3 {
                        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h419) => {
                            h419
                        }
                    }
                } else {
                    v1
                }
            } else {
                v1
            }
        }
        Adt4::C1(_, _) => {
            v1
        }
        Adt4::C2(_, _) => {
            v1
        }
    }
}

pub fn f52(v0: u64, v1: u64) -> u32 {
    let (_, h420) = {
        let (_, h421) = f67(Fn11::F68, v1, (v0, (1u32, 0u32)));
        h421
    };
    h420
}

pub fn f53() -> u32 {
    1u32
}

pub fn f54(v0: u64) -> u32 {
    f52({
        let a422 = v0;
        let a423 = 4294967296u64;
        let a424 = 0u64;
        nat_rem(a422, a423, a424)
    }, 32u64)
}

pub fn f55(v0: u64) -> u32 {
    let a425 = v0;
    let a426 = 4294967296u64;
    if nat_lt(a425, a426) {
        0u32
    } else {
        f52({
            let a427 = v0;
            let a428 = 4294967296u64;
            let a429 = 0u64;
            nat_quot(a427, a428, a429)
        }, 32u64)
    }
}

pub fn f56(v0: Adt3, v1: Adt6, v2: u32, v3: u32, v4: u32, v5: u32) -> R<Adt6> {
    Ok(f69(f70(v0, v1, v2, v3, v4, v5)?))
}

pub fn f57() -> u32 {
    4u32
}

pub fn f58() -> u32 {
    8u32
}

pub fn f59(v0: Adt3, v1: u32, v2: Adt10) -> R<(List<Adt7>, List<Adt4>)> {
    let v3: Adt10 = {
        let (h430, _) = f71(Fn12::F72(v1, v0), 64u64, v2)?;
        h430
    };
    let v4: List<Adt7> = List::cons(match v3.clone() {
        Adt10::C0(h431, _, _, _) => {
            h431
        }
    }, match v3.clone() {
        Adt10::C0(_, _, h432, _) => {
            h432
        }
    });
    Ok((v4.clone(), List::cons(Adt4::C1(match v3.clone() {
        Adt10::C0(h433, _, _, _) => {
            h433
        }
    }, {
        let a434 = v4;
        length_list(a434)
    }), match v3 {
        Adt10::C0(_, _, _, h435) => {
            h435
        }
    })))
}

pub fn f60(v0: u32) -> Bytes {
    let a438 = {
        let a436 = v0;
        let a437 = 128u32;
        fixed_u32::bit_and(a436, a437)
    };
    let a439 = 0u32;
    if equal(a438, a439) {
        let a442 = {
            let a440 = v0;
            let a441 = 64u32;
            fixed_u32::bit_and(a440, a441)
        };
        let a443 = 0u32;
        if equal(a442, a443) {
            let a446 = {
                let a444 = v0;
                let a445 = 32u32;
                fixed_u32::bit_and(a444, a445)
            };
            let a447 = 0u32;
            if equal(a446, a447) {
                let a450 = {
                    let a448 = v0;
                    let a449 = 16u32;
                    fixed_u32::bit_and(a448, a449)
                };
                let a451 = 0u32;
                if equal(a450, a451) {
                    let a454 = {
                        let a452 = v0;
                        let a453 = 8u32;
                        fixed_u32::bit_and(a452, a453)
                    };
                    let a455 = 0u32;
                    if equal(a454, a455) {
                        let a458 = {
                            let a456 = v0;
                            let a457 = 4u32;
                            fixed_u32::bit_and(a456, a457)
                        };
                        let a459 = 0u32;
                        if equal(a458, a459) {
                            let a462 = {
                                let a460 = v0;
                                let a461 = 2u32;
                                fixed_u32::bit_and(a460, a461)
                            };
                            let a463 = 0u32;
                            if equal(a462, a463) {
                                let a466 = {
                                    let a464 = v0;
                                    let a465 = 1u32;
                                    fixed_u32::bit_and(a464, a465)
                                };
                                let a467 = 0u32;
                                if equal(a466, a467) {
                                    Bytes::lit(&[0x00])
                                } else {
                                    Bytes::lit(&[0x01])
                                }
                            } else {
                                let a470 = {
                                    let a468 = v0;
                                    let a469 = 1u32;
                                    fixed_u32::bit_and(a468, a469)
                                };
                                let a471 = 0u32;
                                if equal(a470, a471) {
                                    Bytes::lit(&[0x02])
                                } else {
                                    Bytes::lit(&[0x03])
                                }
                            }
                        } else {
                            let a474 = {
                                let a472 = v0;
                                let a473 = 2u32;
                                fixed_u32::bit_and(a472, a473)
                            };
                            let a475 = 0u32;
                            if equal(a474, a475) {
                                let a478 = {
                                    let a476 = v0;
                                    let a477 = 1u32;
                                    fixed_u32::bit_and(a476, a477)
                                };
                                let a479 = 0u32;
                                if equal(a478, a479) {
                                    Bytes::lit(&[0x04])
                                } else {
                                    Bytes::lit(&[0x05])
                                }
                            } else {
                                let a482 = {
                                    let a480 = v0;
                                    let a481 = 1u32;
                                    fixed_u32::bit_and(a480, a481)
                                };
                                let a483 = 0u32;
                                if equal(a482, a483) {
                                    Bytes::lit(&[0x06])
                                } else {
                                    Bytes::lit(&[0x07])
                                }
                            }
                        }
                    } else {
                        let a486 = {
                            let a484 = v0;
                            let a485 = 4u32;
                            fixed_u32::bit_and(a484, a485)
                        };
                        let a487 = 0u32;
                        if equal(a486, a487) {
                            let a490 = {
                                let a488 = v0;
                                let a489 = 2u32;
                                fixed_u32::bit_and(a488, a489)
                            };
                            let a491 = 0u32;
                            if equal(a490, a491) {
                                let a494 = {
                                    let a492 = v0;
                                    let a493 = 1u32;
                                    fixed_u32::bit_and(a492, a493)
                                };
                                let a495 = 0u32;
                                if equal(a494, a495) {
                                    Bytes::lit(&[0x08])
                                } else {
                                    Bytes::lit(&[0x09])
                                }
                            } else {
                                let a498 = {
                                    let a496 = v0;
                                    let a497 = 1u32;
                                    fixed_u32::bit_and(a496, a497)
                                };
                                let a499 = 0u32;
                                if equal(a498, a499) {
                                    Bytes::lit(&[0x0a])
                                } else {
                                    Bytes::lit(&[0x0b])
                                }
                            }
                        } else {
                            let a502 = {
                                let a500 = v0;
                                let a501 = 2u32;
                                fixed_u32::bit_and(a500, a501)
                            };
                            let a503 = 0u32;
                            if equal(a502, a503) {
                                let a506 = {
                                    let a504 = v0;
                                    let a505 = 1u32;
                                    fixed_u32::bit_and(a504, a505)
                                };
                                let a507 = 0u32;
                                if equal(a506, a507) {
                                    Bytes::lit(&[0x0c])
                                } else {
                                    Bytes::lit(&[0x0d])
                                }
                            } else {
                                let a510 = {
                                    let a508 = v0;
                                    let a509 = 1u32;
                                    fixed_u32::bit_and(a508, a509)
                                };
                                let a511 = 0u32;
                                if equal(a510, a511) {
                                    Bytes::lit(&[0x0e])
                                } else {
                                    Bytes::lit(&[0x0f])
                                }
                            }
                        }
                    }
                } else {
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
                                    Bytes::lit(&[0x10])
                                } else {
                                    Bytes::lit(&[0x11])
                                }
                            } else {
                                let a530 = {
                                    let a528 = v0;
                                    let a529 = 1u32;
                                    fixed_u32::bit_and(a528, a529)
                                };
                                let a531 = 0u32;
                                if equal(a530, a531) {
                                    Bytes::lit(&[0x12])
                                } else {
                                    Bytes::lit(&[0x13])
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
                                    Bytes::lit(&[0x14])
                                } else {
                                    Bytes::lit(&[0x15])
                                }
                            } else {
                                let a542 = {
                                    let a540 = v0;
                                    let a541 = 1u32;
                                    fixed_u32::bit_and(a540, a541)
                                };
                                let a543 = 0u32;
                                if equal(a542, a543) {
                                    Bytes::lit(&[0x16])
                                } else {
                                    Bytes::lit(&[0x17])
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
                                    Bytes::lit(&[0x18])
                                } else {
                                    Bytes::lit(&[0x19])
                                }
                            } else {
                                let a558 = {
                                    let a556 = v0;
                                    let a557 = 1u32;
                                    fixed_u32::bit_and(a556, a557)
                                };
                                let a559 = 0u32;
                                if equal(a558, a559) {
                                    Bytes::lit(&[0x1a])
                                } else {
                                    Bytes::lit(&[0x1b])
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
                                    Bytes::lit(&[0x1c])
                                } else {
                                    Bytes::lit(&[0x1d])
                                }
                            } else {
                                let a570 = {
                                    let a568 = v0;
                                    let a569 = 1u32;
                                    fixed_u32::bit_and(a568, a569)
                                };
                                let a571 = 0u32;
                                if equal(a570, a571) {
                                    Bytes::lit(&[0x1e])
                                } else {
                                    Bytes::lit(&[0x1f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a574 = {
                    let a572 = v0;
                    let a573 = 16u32;
                    fixed_u32::bit_and(a572, a573)
                };
                let a575 = 0u32;
                if equal(a574, a575) {
                    let a578 = {
                        let a576 = v0;
                        let a577 = 8u32;
                        fixed_u32::bit_and(a576, a577)
                    };
                    let a579 = 0u32;
                    if equal(a578, a579) {
                        let a582 = {
                            let a580 = v0;
                            let a581 = 4u32;
                            fixed_u32::bit_and(a580, a581)
                        };
                        let a583 = 0u32;
                        if equal(a582, a583) {
                            let a586 = {
                                let a584 = v0;
                                let a585 = 2u32;
                                fixed_u32::bit_and(a584, a585)
                            };
                            let a587 = 0u32;
                            if equal(a586, a587) {
                                let a590 = {
                                    let a588 = v0;
                                    let a589 = 1u32;
                                    fixed_u32::bit_and(a588, a589)
                                };
                                let a591 = 0u32;
                                if equal(a590, a591) {
                                    Bytes::lit(&[0x20])
                                } else {
                                    Bytes::lit(&[0x21])
                                }
                            } else {
                                let a594 = {
                                    let a592 = v0;
                                    let a593 = 1u32;
                                    fixed_u32::bit_and(a592, a593)
                                };
                                let a595 = 0u32;
                                if equal(a594, a595) {
                                    Bytes::lit(&[0x22])
                                } else {
                                    Bytes::lit(&[0x23])
                                }
                            }
                        } else {
                            let a598 = {
                                let a596 = v0;
                                let a597 = 2u32;
                                fixed_u32::bit_and(a596, a597)
                            };
                            let a599 = 0u32;
                            if equal(a598, a599) {
                                let a602 = {
                                    let a600 = v0;
                                    let a601 = 1u32;
                                    fixed_u32::bit_and(a600, a601)
                                };
                                let a603 = 0u32;
                                if equal(a602, a603) {
                                    Bytes::lit(&[0x24])
                                } else {
                                    Bytes::lit(&[0x25])
                                }
                            } else {
                                let a606 = {
                                    let a604 = v0;
                                    let a605 = 1u32;
                                    fixed_u32::bit_and(a604, a605)
                                };
                                let a607 = 0u32;
                                if equal(a606, a607) {
                                    Bytes::lit(&[0x26])
                                } else {
                                    Bytes::lit(&[0x27])
                                }
                            }
                        }
                    } else {
                        let a610 = {
                            let a608 = v0;
                            let a609 = 4u32;
                            fixed_u32::bit_and(a608, a609)
                        };
                        let a611 = 0u32;
                        if equal(a610, a611) {
                            let a614 = {
                                let a612 = v0;
                                let a613 = 2u32;
                                fixed_u32::bit_and(a612, a613)
                            };
                            let a615 = 0u32;
                            if equal(a614, a615) {
                                let a618 = {
                                    let a616 = v0;
                                    let a617 = 1u32;
                                    fixed_u32::bit_and(a616, a617)
                                };
                                let a619 = 0u32;
                                if equal(a618, a619) {
                                    Bytes::lit(&[0x28])
                                } else {
                                    Bytes::lit(&[0x29])
                                }
                            } else {
                                let a622 = {
                                    let a620 = v0;
                                    let a621 = 1u32;
                                    fixed_u32::bit_and(a620, a621)
                                };
                                let a623 = 0u32;
                                if equal(a622, a623) {
                                    Bytes::lit(&[0x2a])
                                } else {
                                    Bytes::lit(&[0x2b])
                                }
                            }
                        } else {
                            let a626 = {
                                let a624 = v0;
                                let a625 = 2u32;
                                fixed_u32::bit_and(a624, a625)
                            };
                            let a627 = 0u32;
                            if equal(a626, a627) {
                                let a630 = {
                                    let a628 = v0;
                                    let a629 = 1u32;
                                    fixed_u32::bit_and(a628, a629)
                                };
                                let a631 = 0u32;
                                if equal(a630, a631) {
                                    Bytes::lit(&[0x2c])
                                } else {
                                    Bytes::lit(&[0x2d])
                                }
                            } else {
                                let a634 = {
                                    let a632 = v0;
                                    let a633 = 1u32;
                                    fixed_u32::bit_and(a632, a633)
                                };
                                let a635 = 0u32;
                                if equal(a634, a635) {
                                    Bytes::lit(&[0x2e])
                                } else {
                                    Bytes::lit(&[0x2f])
                                }
                            }
                        }
                    }
                } else {
                    let a638 = {
                        let a636 = v0;
                        let a637 = 8u32;
                        fixed_u32::bit_and(a636, a637)
                    };
                    let a639 = 0u32;
                    if equal(a638, a639) {
                        let a642 = {
                            let a640 = v0;
                            let a641 = 4u32;
                            fixed_u32::bit_and(a640, a641)
                        };
                        let a643 = 0u32;
                        if equal(a642, a643) {
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
                                    Bytes::lit(&[0x30])
                                } else {
                                    Bytes::lit(&[0x31])
                                }
                            } else {
                                let a654 = {
                                    let a652 = v0;
                                    let a653 = 1u32;
                                    fixed_u32::bit_and(a652, a653)
                                };
                                let a655 = 0u32;
                                if equal(a654, a655) {
                                    Bytes::lit(&[0x32])
                                } else {
                                    Bytes::lit(&[0x33])
                                }
                            }
                        } else {
                            let a658 = {
                                let a656 = v0;
                                let a657 = 2u32;
                                fixed_u32::bit_and(a656, a657)
                            };
                            let a659 = 0u32;
                            if equal(a658, a659) {
                                let a662 = {
                                    let a660 = v0;
                                    let a661 = 1u32;
                                    fixed_u32::bit_and(a660, a661)
                                };
                                let a663 = 0u32;
                                if equal(a662, a663) {
                                    Bytes::lit(&[0x34])
                                } else {
                                    Bytes::lit(&[0x35])
                                }
                            } else {
                                let a666 = {
                                    let a664 = v0;
                                    let a665 = 1u32;
                                    fixed_u32::bit_and(a664, a665)
                                };
                                let a667 = 0u32;
                                if equal(a666, a667) {
                                    Bytes::lit(&[0x36])
                                } else {
                                    Bytes::lit(&[0x37])
                                }
                            }
                        }
                    } else {
                        let a670 = {
                            let a668 = v0;
                            let a669 = 4u32;
                            fixed_u32::bit_and(a668, a669)
                        };
                        let a671 = 0u32;
                        if equal(a670, a671) {
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
                                    Bytes::lit(&[0x38])
                                } else {
                                    Bytes::lit(&[0x39])
                                }
                            } else {
                                let a682 = {
                                    let a680 = v0;
                                    let a681 = 1u32;
                                    fixed_u32::bit_and(a680, a681)
                                };
                                let a683 = 0u32;
                                if equal(a682, a683) {
                                    Bytes::lit(&[0x3a])
                                } else {
                                    Bytes::lit(&[0x3b])
                                }
                            }
                        } else {
                            let a686 = {
                                let a684 = v0;
                                let a685 = 2u32;
                                fixed_u32::bit_and(a684, a685)
                            };
                            let a687 = 0u32;
                            if equal(a686, a687) {
                                let a690 = {
                                    let a688 = v0;
                                    let a689 = 1u32;
                                    fixed_u32::bit_and(a688, a689)
                                };
                                let a691 = 0u32;
                                if equal(a690, a691) {
                                    Bytes::lit(&[0x3c])
                                } else {
                                    Bytes::lit(&[0x3d])
                                }
                            } else {
                                let a694 = {
                                    let a692 = v0;
                                    let a693 = 1u32;
                                    fixed_u32::bit_and(a692, a693)
                                };
                                let a695 = 0u32;
                                if equal(a694, a695) {
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
            let a698 = {
                let a696 = v0;
                let a697 = 32u32;
                fixed_u32::bit_and(a696, a697)
            };
            let a699 = 0u32;
            if equal(a698, a699) {
                let a702 = {
                    let a700 = v0;
                    let a701 = 16u32;
                    fixed_u32::bit_and(a700, a701)
                };
                let a703 = 0u32;
                if equal(a702, a703) {
                    let a706 = {
                        let a704 = v0;
                        let a705 = 8u32;
                        fixed_u32::bit_and(a704, a705)
                    };
                    let a707 = 0u32;
                    if equal(a706, a707) {
                        let a710 = {
                            let a708 = v0;
                            let a709 = 4u32;
                            fixed_u32::bit_and(a708, a709)
                        };
                        let a711 = 0u32;
                        if equal(a710, a711) {
                            let a714 = {
                                let a712 = v0;
                                let a713 = 2u32;
                                fixed_u32::bit_and(a712, a713)
                            };
                            let a715 = 0u32;
                            if equal(a714, a715) {
                                let a718 = {
                                    let a716 = v0;
                                    let a717 = 1u32;
                                    fixed_u32::bit_and(a716, a717)
                                };
                                let a719 = 0u32;
                                if equal(a718, a719) {
                                    Bytes::lit(&[0x40])
                                } else {
                                    Bytes::lit(&[0x41])
                                }
                            } else {
                                let a722 = {
                                    let a720 = v0;
                                    let a721 = 1u32;
                                    fixed_u32::bit_and(a720, a721)
                                };
                                let a723 = 0u32;
                                if equal(a722, a723) {
                                    Bytes::lit(&[0x42])
                                } else {
                                    Bytes::lit(&[0x43])
                                }
                            }
                        } else {
                            let a726 = {
                                let a724 = v0;
                                let a725 = 2u32;
                                fixed_u32::bit_and(a724, a725)
                            };
                            let a727 = 0u32;
                            if equal(a726, a727) {
                                let a730 = {
                                    let a728 = v0;
                                    let a729 = 1u32;
                                    fixed_u32::bit_and(a728, a729)
                                };
                                let a731 = 0u32;
                                if equal(a730, a731) {
                                    Bytes::lit(&[0x44])
                                } else {
                                    Bytes::lit(&[0x45])
                                }
                            } else {
                                let a734 = {
                                    let a732 = v0;
                                    let a733 = 1u32;
                                    fixed_u32::bit_and(a732, a733)
                                };
                                let a735 = 0u32;
                                if equal(a734, a735) {
                                    Bytes::lit(&[0x46])
                                } else {
                                    Bytes::lit(&[0x47])
                                }
                            }
                        }
                    } else {
                        let a738 = {
                            let a736 = v0;
                            let a737 = 4u32;
                            fixed_u32::bit_and(a736, a737)
                        };
                        let a739 = 0u32;
                        if equal(a738, a739) {
                            let a742 = {
                                let a740 = v0;
                                let a741 = 2u32;
                                fixed_u32::bit_and(a740, a741)
                            };
                            let a743 = 0u32;
                            if equal(a742, a743) {
                                let a746 = {
                                    let a744 = v0;
                                    let a745 = 1u32;
                                    fixed_u32::bit_and(a744, a745)
                                };
                                let a747 = 0u32;
                                if equal(a746, a747) {
                                    Bytes::lit(&[0x48])
                                } else {
                                    Bytes::lit(&[0x49])
                                }
                            } else {
                                let a750 = {
                                    let a748 = v0;
                                    let a749 = 1u32;
                                    fixed_u32::bit_and(a748, a749)
                                };
                                let a751 = 0u32;
                                if equal(a750, a751) {
                                    Bytes::lit(&[0x4a])
                                } else {
                                    Bytes::lit(&[0x4b])
                                }
                            }
                        } else {
                            let a754 = {
                                let a752 = v0;
                                let a753 = 2u32;
                                fixed_u32::bit_and(a752, a753)
                            };
                            let a755 = 0u32;
                            if equal(a754, a755) {
                                let a758 = {
                                    let a756 = v0;
                                    let a757 = 1u32;
                                    fixed_u32::bit_and(a756, a757)
                                };
                                let a759 = 0u32;
                                if equal(a758, a759) {
                                    Bytes::lit(&[0x4c])
                                } else {
                                    Bytes::lit(&[0x4d])
                                }
                            } else {
                                let a762 = {
                                    let a760 = v0;
                                    let a761 = 1u32;
                                    fixed_u32::bit_and(a760, a761)
                                };
                                let a763 = 0u32;
                                if equal(a762, a763) {
                                    Bytes::lit(&[0x4e])
                                } else {
                                    Bytes::lit(&[0x4f])
                                }
                            }
                        }
                    }
                } else {
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
                                    Bytes::lit(&[0x50])
                                } else {
                                    Bytes::lit(&[0x51])
                                }
                            } else {
                                let a782 = {
                                    let a780 = v0;
                                    let a781 = 1u32;
                                    fixed_u32::bit_and(a780, a781)
                                };
                                let a783 = 0u32;
                                if equal(a782, a783) {
                                    Bytes::lit(&[0x52])
                                } else {
                                    Bytes::lit(&[0x53])
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
                                    Bytes::lit(&[0x54])
                                } else {
                                    Bytes::lit(&[0x55])
                                }
                            } else {
                                let a794 = {
                                    let a792 = v0;
                                    let a793 = 1u32;
                                    fixed_u32::bit_and(a792, a793)
                                };
                                let a795 = 0u32;
                                if equal(a794, a795) {
                                    Bytes::lit(&[0x56])
                                } else {
                                    Bytes::lit(&[0x57])
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
                                    Bytes::lit(&[0x58])
                                } else {
                                    Bytes::lit(&[0x59])
                                }
                            } else {
                                let a810 = {
                                    let a808 = v0;
                                    let a809 = 1u32;
                                    fixed_u32::bit_and(a808, a809)
                                };
                                let a811 = 0u32;
                                if equal(a810, a811) {
                                    Bytes::lit(&[0x5a])
                                } else {
                                    Bytes::lit(&[0x5b])
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
                                    Bytes::lit(&[0x5c])
                                } else {
                                    Bytes::lit(&[0x5d])
                                }
                            } else {
                                let a822 = {
                                    let a820 = v0;
                                    let a821 = 1u32;
                                    fixed_u32::bit_and(a820, a821)
                                };
                                let a823 = 0u32;
                                if equal(a822, a823) {
                                    Bytes::lit(&[0x5e])
                                } else {
                                    Bytes::lit(&[0x5f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a826 = {
                    let a824 = v0;
                    let a825 = 16u32;
                    fixed_u32::bit_and(a824, a825)
                };
                let a827 = 0u32;
                if equal(a826, a827) {
                    let a830 = {
                        let a828 = v0;
                        let a829 = 8u32;
                        fixed_u32::bit_and(a828, a829)
                    };
                    let a831 = 0u32;
                    if equal(a830, a831) {
                        let a834 = {
                            let a832 = v0;
                            let a833 = 4u32;
                            fixed_u32::bit_and(a832, a833)
                        };
                        let a835 = 0u32;
                        if equal(a834, a835) {
                            let a838 = {
                                let a836 = v0;
                                let a837 = 2u32;
                                fixed_u32::bit_and(a836, a837)
                            };
                            let a839 = 0u32;
                            if equal(a838, a839) {
                                let a842 = {
                                    let a840 = v0;
                                    let a841 = 1u32;
                                    fixed_u32::bit_and(a840, a841)
                                };
                                let a843 = 0u32;
                                if equal(a842, a843) {
                                    Bytes::lit(&[0x60])
                                } else {
                                    Bytes::lit(&[0x61])
                                }
                            } else {
                                let a846 = {
                                    let a844 = v0;
                                    let a845 = 1u32;
                                    fixed_u32::bit_and(a844, a845)
                                };
                                let a847 = 0u32;
                                if equal(a846, a847) {
                                    Bytes::lit(&[0x62])
                                } else {
                                    Bytes::lit(&[0x63])
                                }
                            }
                        } else {
                            let a850 = {
                                let a848 = v0;
                                let a849 = 2u32;
                                fixed_u32::bit_and(a848, a849)
                            };
                            let a851 = 0u32;
                            if equal(a850, a851) {
                                let a854 = {
                                    let a852 = v0;
                                    let a853 = 1u32;
                                    fixed_u32::bit_and(a852, a853)
                                };
                                let a855 = 0u32;
                                if equal(a854, a855) {
                                    Bytes::lit(&[0x64])
                                } else {
                                    Bytes::lit(&[0x65])
                                }
                            } else {
                                let a858 = {
                                    let a856 = v0;
                                    let a857 = 1u32;
                                    fixed_u32::bit_and(a856, a857)
                                };
                                let a859 = 0u32;
                                if equal(a858, a859) {
                                    Bytes::lit(&[0x66])
                                } else {
                                    Bytes::lit(&[0x67])
                                }
                            }
                        }
                    } else {
                        let a862 = {
                            let a860 = v0;
                            let a861 = 4u32;
                            fixed_u32::bit_and(a860, a861)
                        };
                        let a863 = 0u32;
                        if equal(a862, a863) {
                            let a866 = {
                                let a864 = v0;
                                let a865 = 2u32;
                                fixed_u32::bit_and(a864, a865)
                            };
                            let a867 = 0u32;
                            if equal(a866, a867) {
                                let a870 = {
                                    let a868 = v0;
                                    let a869 = 1u32;
                                    fixed_u32::bit_and(a868, a869)
                                };
                                let a871 = 0u32;
                                if equal(a870, a871) {
                                    Bytes::lit(&[0x68])
                                } else {
                                    Bytes::lit(&[0x69])
                                }
                            } else {
                                let a874 = {
                                    let a872 = v0;
                                    let a873 = 1u32;
                                    fixed_u32::bit_and(a872, a873)
                                };
                                let a875 = 0u32;
                                if equal(a874, a875) {
                                    Bytes::lit(&[0x6a])
                                } else {
                                    Bytes::lit(&[0x6b])
                                }
                            }
                        } else {
                            let a878 = {
                                let a876 = v0;
                                let a877 = 2u32;
                                fixed_u32::bit_and(a876, a877)
                            };
                            let a879 = 0u32;
                            if equal(a878, a879) {
                                let a882 = {
                                    let a880 = v0;
                                    let a881 = 1u32;
                                    fixed_u32::bit_and(a880, a881)
                                };
                                let a883 = 0u32;
                                if equal(a882, a883) {
                                    Bytes::lit(&[0x6c])
                                } else {
                                    Bytes::lit(&[0x6d])
                                }
                            } else {
                                let a886 = {
                                    let a884 = v0;
                                    let a885 = 1u32;
                                    fixed_u32::bit_and(a884, a885)
                                };
                                let a887 = 0u32;
                                if equal(a886, a887) {
                                    Bytes::lit(&[0x6e])
                                } else {
                                    Bytes::lit(&[0x6f])
                                }
                            }
                        }
                    }
                } else {
                    let a890 = {
                        let a888 = v0;
                        let a889 = 8u32;
                        fixed_u32::bit_and(a888, a889)
                    };
                    let a891 = 0u32;
                    if equal(a890, a891) {
                        let a894 = {
                            let a892 = v0;
                            let a893 = 4u32;
                            fixed_u32::bit_and(a892, a893)
                        };
                        let a895 = 0u32;
                        if equal(a894, a895) {
                            let a898 = {
                                let a896 = v0;
                                let a897 = 2u32;
                                fixed_u32::bit_and(a896, a897)
                            };
                            let a899 = 0u32;
                            if equal(a898, a899) {
                                let a902 = {
                                    let a900 = v0;
                                    let a901 = 1u32;
                                    fixed_u32::bit_and(a900, a901)
                                };
                                let a903 = 0u32;
                                if equal(a902, a903) {
                                    Bytes::lit(&[0x70])
                                } else {
                                    Bytes::lit(&[0x71])
                                }
                            } else {
                                let a906 = {
                                    let a904 = v0;
                                    let a905 = 1u32;
                                    fixed_u32::bit_and(a904, a905)
                                };
                                let a907 = 0u32;
                                if equal(a906, a907) {
                                    Bytes::lit(&[0x72])
                                } else {
                                    Bytes::lit(&[0x73])
                                }
                            }
                        } else {
                            let a910 = {
                                let a908 = v0;
                                let a909 = 2u32;
                                fixed_u32::bit_and(a908, a909)
                            };
                            let a911 = 0u32;
                            if equal(a910, a911) {
                                let a914 = {
                                    let a912 = v0;
                                    let a913 = 1u32;
                                    fixed_u32::bit_and(a912, a913)
                                };
                                let a915 = 0u32;
                                if equal(a914, a915) {
                                    Bytes::lit(&[0x74])
                                } else {
                                    Bytes::lit(&[0x75])
                                }
                            } else {
                                let a918 = {
                                    let a916 = v0;
                                    let a917 = 1u32;
                                    fixed_u32::bit_and(a916, a917)
                                };
                                let a919 = 0u32;
                                if equal(a918, a919) {
                                    Bytes::lit(&[0x76])
                                } else {
                                    Bytes::lit(&[0x77])
                                }
                            }
                        }
                    } else {
                        let a922 = {
                            let a920 = v0;
                            let a921 = 4u32;
                            fixed_u32::bit_and(a920, a921)
                        };
                        let a923 = 0u32;
                        if equal(a922, a923) {
                            let a926 = {
                                let a924 = v0;
                                let a925 = 2u32;
                                fixed_u32::bit_and(a924, a925)
                            };
                            let a927 = 0u32;
                            if equal(a926, a927) {
                                let a930 = {
                                    let a928 = v0;
                                    let a929 = 1u32;
                                    fixed_u32::bit_and(a928, a929)
                                };
                                let a931 = 0u32;
                                if equal(a930, a931) {
                                    Bytes::lit(&[0x78])
                                } else {
                                    Bytes::lit(&[0x79])
                                }
                            } else {
                                let a934 = {
                                    let a932 = v0;
                                    let a933 = 1u32;
                                    fixed_u32::bit_and(a932, a933)
                                };
                                let a935 = 0u32;
                                if equal(a934, a935) {
                                    Bytes::lit(&[0x7a])
                                } else {
                                    Bytes::lit(&[0x7b])
                                }
                            }
                        } else {
                            let a938 = {
                                let a936 = v0;
                                let a937 = 2u32;
                                fixed_u32::bit_and(a936, a937)
                            };
                            let a939 = 0u32;
                            if equal(a938, a939) {
                                let a942 = {
                                    let a940 = v0;
                                    let a941 = 1u32;
                                    fixed_u32::bit_and(a940, a941)
                                };
                                let a943 = 0u32;
                                if equal(a942, a943) {
                                    Bytes::lit(&[0x7c])
                                } else {
                                    Bytes::lit(&[0x7d])
                                }
                            } else {
                                let a946 = {
                                    let a944 = v0;
                                    let a945 = 1u32;
                                    fixed_u32::bit_and(a944, a945)
                                };
                                let a947 = 0u32;
                                if equal(a946, a947) {
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
        let a950 = {
            let a948 = v0;
            let a949 = 64u32;
            fixed_u32::bit_and(a948, a949)
        };
        let a951 = 0u32;
        if equal(a950, a951) {
            let a954 = {
                let a952 = v0;
                let a953 = 32u32;
                fixed_u32::bit_and(a952, a953)
            };
            let a955 = 0u32;
            if equal(a954, a955) {
                let a958 = {
                    let a956 = v0;
                    let a957 = 16u32;
                    fixed_u32::bit_and(a956, a957)
                };
                let a959 = 0u32;
                if equal(a958, a959) {
                    let a962 = {
                        let a960 = v0;
                        let a961 = 8u32;
                        fixed_u32::bit_and(a960, a961)
                    };
                    let a963 = 0u32;
                    if equal(a962, a963) {
                        let a966 = {
                            let a964 = v0;
                            let a965 = 4u32;
                            fixed_u32::bit_and(a964, a965)
                        };
                        let a967 = 0u32;
                        if equal(a966, a967) {
                            let a970 = {
                                let a968 = v0;
                                let a969 = 2u32;
                                fixed_u32::bit_and(a968, a969)
                            };
                            let a971 = 0u32;
                            if equal(a970, a971) {
                                let a974 = {
                                    let a972 = v0;
                                    let a973 = 1u32;
                                    fixed_u32::bit_and(a972, a973)
                                };
                                let a975 = 0u32;
                                if equal(a974, a975) {
                                    Bytes::lit(&[0x80])
                                } else {
                                    Bytes::lit(&[0x81])
                                }
                            } else {
                                let a978 = {
                                    let a976 = v0;
                                    let a977 = 1u32;
                                    fixed_u32::bit_and(a976, a977)
                                };
                                let a979 = 0u32;
                                if equal(a978, a979) {
                                    Bytes::lit(&[0x82])
                                } else {
                                    Bytes::lit(&[0x83])
                                }
                            }
                        } else {
                            let a982 = {
                                let a980 = v0;
                                let a981 = 2u32;
                                fixed_u32::bit_and(a980, a981)
                            };
                            let a983 = 0u32;
                            if equal(a982, a983) {
                                let a986 = {
                                    let a984 = v0;
                                    let a985 = 1u32;
                                    fixed_u32::bit_and(a984, a985)
                                };
                                let a987 = 0u32;
                                if equal(a986, a987) {
                                    Bytes::lit(&[0x84])
                                } else {
                                    Bytes::lit(&[0x85])
                                }
                            } else {
                                let a990 = {
                                    let a988 = v0;
                                    let a989 = 1u32;
                                    fixed_u32::bit_and(a988, a989)
                                };
                                let a991 = 0u32;
                                if equal(a990, a991) {
                                    Bytes::lit(&[0x86])
                                } else {
                                    Bytes::lit(&[0x87])
                                }
                            }
                        }
                    } else {
                        let a994 = {
                            let a992 = v0;
                            let a993 = 4u32;
                            fixed_u32::bit_and(a992, a993)
                        };
                        let a995 = 0u32;
                        if equal(a994, a995) {
                            let a998 = {
                                let a996 = v0;
                                let a997 = 2u32;
                                fixed_u32::bit_and(a996, a997)
                            };
                            let a999 = 0u32;
                            if equal(a998, a999) {
                                let a1002 = {
                                    let a1000 = v0;
                                    let a1001 = 1u32;
                                    fixed_u32::bit_and(a1000, a1001)
                                };
                                let a1003 = 0u32;
                                if equal(a1002, a1003) {
                                    Bytes::lit(&[0x88])
                                } else {
                                    Bytes::lit(&[0x89])
                                }
                            } else {
                                let a1006 = {
                                    let a1004 = v0;
                                    let a1005 = 1u32;
                                    fixed_u32::bit_and(a1004, a1005)
                                };
                                let a1007 = 0u32;
                                if equal(a1006, a1007) {
                                    Bytes::lit(&[0x8a])
                                } else {
                                    Bytes::lit(&[0x8b])
                                }
                            }
                        } else {
                            let a1010 = {
                                let a1008 = v0;
                                let a1009 = 2u32;
                                fixed_u32::bit_and(a1008, a1009)
                            };
                            let a1011 = 0u32;
                            if equal(a1010, a1011) {
                                let a1014 = {
                                    let a1012 = v0;
                                    let a1013 = 1u32;
                                    fixed_u32::bit_and(a1012, a1013)
                                };
                                let a1015 = 0u32;
                                if equal(a1014, a1015) {
                                    Bytes::lit(&[0x8c])
                                } else {
                                    Bytes::lit(&[0x8d])
                                }
                            } else {
                                let a1018 = {
                                    let a1016 = v0;
                                    let a1017 = 1u32;
                                    fixed_u32::bit_and(a1016, a1017)
                                };
                                let a1019 = 0u32;
                                if equal(a1018, a1019) {
                                    Bytes::lit(&[0x8e])
                                } else {
                                    Bytes::lit(&[0x8f])
                                }
                            }
                        }
                    }
                } else {
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
                                    Bytes::lit(&[0x90])
                                } else {
                                    Bytes::lit(&[0x91])
                                }
                            } else {
                                let a1038 = {
                                    let a1036 = v0;
                                    let a1037 = 1u32;
                                    fixed_u32::bit_and(a1036, a1037)
                                };
                                let a1039 = 0u32;
                                if equal(a1038, a1039) {
                                    Bytes::lit(&[0x92])
                                } else {
                                    Bytes::lit(&[0x93])
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
                                    Bytes::lit(&[0x94])
                                } else {
                                    Bytes::lit(&[0x95])
                                }
                            } else {
                                let a1050 = {
                                    let a1048 = v0;
                                    let a1049 = 1u32;
                                    fixed_u32::bit_and(a1048, a1049)
                                };
                                let a1051 = 0u32;
                                if equal(a1050, a1051) {
                                    Bytes::lit(&[0x96])
                                } else {
                                    Bytes::lit(&[0x97])
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
                                    Bytes::lit(&[0x98])
                                } else {
                                    Bytes::lit(&[0x99])
                                }
                            } else {
                                let a1066 = {
                                    let a1064 = v0;
                                    let a1065 = 1u32;
                                    fixed_u32::bit_and(a1064, a1065)
                                };
                                let a1067 = 0u32;
                                if equal(a1066, a1067) {
                                    Bytes::lit(&[0x9a])
                                } else {
                                    Bytes::lit(&[0x9b])
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
                                    Bytes::lit(&[0x9c])
                                } else {
                                    Bytes::lit(&[0x9d])
                                }
                            } else {
                                let a1078 = {
                                    let a1076 = v0;
                                    let a1077 = 1u32;
                                    fixed_u32::bit_and(a1076, a1077)
                                };
                                let a1079 = 0u32;
                                if equal(a1078, a1079) {
                                    Bytes::lit(&[0x9e])
                                } else {
                                    Bytes::lit(&[0x9f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a1082 = {
                    let a1080 = v0;
                    let a1081 = 16u32;
                    fixed_u32::bit_and(a1080, a1081)
                };
                let a1083 = 0u32;
                if equal(a1082, a1083) {
                    let a1086 = {
                        let a1084 = v0;
                        let a1085 = 8u32;
                        fixed_u32::bit_and(a1084, a1085)
                    };
                    let a1087 = 0u32;
                    if equal(a1086, a1087) {
                        let a1090 = {
                            let a1088 = v0;
                            let a1089 = 4u32;
                            fixed_u32::bit_and(a1088, a1089)
                        };
                        let a1091 = 0u32;
                        if equal(a1090, a1091) {
                            let a1094 = {
                                let a1092 = v0;
                                let a1093 = 2u32;
                                fixed_u32::bit_and(a1092, a1093)
                            };
                            let a1095 = 0u32;
                            if equal(a1094, a1095) {
                                let a1098 = {
                                    let a1096 = v0;
                                    let a1097 = 1u32;
                                    fixed_u32::bit_and(a1096, a1097)
                                };
                                let a1099 = 0u32;
                                if equal(a1098, a1099) {
                                    Bytes::lit(&[0xa0])
                                } else {
                                    Bytes::lit(&[0xa1])
                                }
                            } else {
                                let a1102 = {
                                    let a1100 = v0;
                                    let a1101 = 1u32;
                                    fixed_u32::bit_and(a1100, a1101)
                                };
                                let a1103 = 0u32;
                                if equal(a1102, a1103) {
                                    Bytes::lit(&[0xa2])
                                } else {
                                    Bytes::lit(&[0xa3])
                                }
                            }
                        } else {
                            let a1106 = {
                                let a1104 = v0;
                                let a1105 = 2u32;
                                fixed_u32::bit_and(a1104, a1105)
                            };
                            let a1107 = 0u32;
                            if equal(a1106, a1107) {
                                let a1110 = {
                                    let a1108 = v0;
                                    let a1109 = 1u32;
                                    fixed_u32::bit_and(a1108, a1109)
                                };
                                let a1111 = 0u32;
                                if equal(a1110, a1111) {
                                    Bytes::lit(&[0xa4])
                                } else {
                                    Bytes::lit(&[0xa5])
                                }
                            } else {
                                let a1114 = {
                                    let a1112 = v0;
                                    let a1113 = 1u32;
                                    fixed_u32::bit_and(a1112, a1113)
                                };
                                let a1115 = 0u32;
                                if equal(a1114, a1115) {
                                    Bytes::lit(&[0xa6])
                                } else {
                                    Bytes::lit(&[0xa7])
                                }
                            }
                        }
                    } else {
                        let a1118 = {
                            let a1116 = v0;
                            let a1117 = 4u32;
                            fixed_u32::bit_and(a1116, a1117)
                        };
                        let a1119 = 0u32;
                        if equal(a1118, a1119) {
                            let a1122 = {
                                let a1120 = v0;
                                let a1121 = 2u32;
                                fixed_u32::bit_and(a1120, a1121)
                            };
                            let a1123 = 0u32;
                            if equal(a1122, a1123) {
                                let a1126 = {
                                    let a1124 = v0;
                                    let a1125 = 1u32;
                                    fixed_u32::bit_and(a1124, a1125)
                                };
                                let a1127 = 0u32;
                                if equal(a1126, a1127) {
                                    Bytes::lit(&[0xa8])
                                } else {
                                    Bytes::lit(&[0xa9])
                                }
                            } else {
                                let a1130 = {
                                    let a1128 = v0;
                                    let a1129 = 1u32;
                                    fixed_u32::bit_and(a1128, a1129)
                                };
                                let a1131 = 0u32;
                                if equal(a1130, a1131) {
                                    Bytes::lit(&[0xaa])
                                } else {
                                    Bytes::lit(&[0xab])
                                }
                            }
                        } else {
                            let a1134 = {
                                let a1132 = v0;
                                let a1133 = 2u32;
                                fixed_u32::bit_and(a1132, a1133)
                            };
                            let a1135 = 0u32;
                            if equal(a1134, a1135) {
                                let a1138 = {
                                    let a1136 = v0;
                                    let a1137 = 1u32;
                                    fixed_u32::bit_and(a1136, a1137)
                                };
                                let a1139 = 0u32;
                                if equal(a1138, a1139) {
                                    Bytes::lit(&[0xac])
                                } else {
                                    Bytes::lit(&[0xad])
                                }
                            } else {
                                let a1142 = {
                                    let a1140 = v0;
                                    let a1141 = 1u32;
                                    fixed_u32::bit_and(a1140, a1141)
                                };
                                let a1143 = 0u32;
                                if equal(a1142, a1143) {
                                    Bytes::lit(&[0xae])
                                } else {
                                    Bytes::lit(&[0xaf])
                                }
                            }
                        }
                    }
                } else {
                    let a1146 = {
                        let a1144 = v0;
                        let a1145 = 8u32;
                        fixed_u32::bit_and(a1144, a1145)
                    };
                    let a1147 = 0u32;
                    if equal(a1146, a1147) {
                        let a1150 = {
                            let a1148 = v0;
                            let a1149 = 4u32;
                            fixed_u32::bit_and(a1148, a1149)
                        };
                        let a1151 = 0u32;
                        if equal(a1150, a1151) {
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
                                    Bytes::lit(&[0xb0])
                                } else {
                                    Bytes::lit(&[0xb1])
                                }
                            } else {
                                let a1162 = {
                                    let a1160 = v0;
                                    let a1161 = 1u32;
                                    fixed_u32::bit_and(a1160, a1161)
                                };
                                let a1163 = 0u32;
                                if equal(a1162, a1163) {
                                    Bytes::lit(&[0xb2])
                                } else {
                                    Bytes::lit(&[0xb3])
                                }
                            }
                        } else {
                            let a1166 = {
                                let a1164 = v0;
                                let a1165 = 2u32;
                                fixed_u32::bit_and(a1164, a1165)
                            };
                            let a1167 = 0u32;
                            if equal(a1166, a1167) {
                                let a1170 = {
                                    let a1168 = v0;
                                    let a1169 = 1u32;
                                    fixed_u32::bit_and(a1168, a1169)
                                };
                                let a1171 = 0u32;
                                if equal(a1170, a1171) {
                                    Bytes::lit(&[0xb4])
                                } else {
                                    Bytes::lit(&[0xb5])
                                }
                            } else {
                                let a1174 = {
                                    let a1172 = v0;
                                    let a1173 = 1u32;
                                    fixed_u32::bit_and(a1172, a1173)
                                };
                                let a1175 = 0u32;
                                if equal(a1174, a1175) {
                                    Bytes::lit(&[0xb6])
                                } else {
                                    Bytes::lit(&[0xb7])
                                }
                            }
                        }
                    } else {
                        let a1178 = {
                            let a1176 = v0;
                            let a1177 = 4u32;
                            fixed_u32::bit_and(a1176, a1177)
                        };
                        let a1179 = 0u32;
                        if equal(a1178, a1179) {
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
                                    Bytes::lit(&[0xb8])
                                } else {
                                    Bytes::lit(&[0xb9])
                                }
                            } else {
                                let a1190 = {
                                    let a1188 = v0;
                                    let a1189 = 1u32;
                                    fixed_u32::bit_and(a1188, a1189)
                                };
                                let a1191 = 0u32;
                                if equal(a1190, a1191) {
                                    Bytes::lit(&[0xba])
                                } else {
                                    Bytes::lit(&[0xbb])
                                }
                            }
                        } else {
                            let a1194 = {
                                let a1192 = v0;
                                let a1193 = 2u32;
                                fixed_u32::bit_and(a1192, a1193)
                            };
                            let a1195 = 0u32;
                            if equal(a1194, a1195) {
                                let a1198 = {
                                    let a1196 = v0;
                                    let a1197 = 1u32;
                                    fixed_u32::bit_and(a1196, a1197)
                                };
                                let a1199 = 0u32;
                                if equal(a1198, a1199) {
                                    Bytes::lit(&[0xbc])
                                } else {
                                    Bytes::lit(&[0xbd])
                                }
                            } else {
                                let a1202 = {
                                    let a1200 = v0;
                                    let a1201 = 1u32;
                                    fixed_u32::bit_and(a1200, a1201)
                                };
                                let a1203 = 0u32;
                                if equal(a1202, a1203) {
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
            let a1206 = {
                let a1204 = v0;
                let a1205 = 32u32;
                fixed_u32::bit_and(a1204, a1205)
            };
            let a1207 = 0u32;
            if equal(a1206, a1207) {
                let a1210 = {
                    let a1208 = v0;
                    let a1209 = 16u32;
                    fixed_u32::bit_and(a1208, a1209)
                };
                let a1211 = 0u32;
                if equal(a1210, a1211) {
                    let a1214 = {
                        let a1212 = v0;
                        let a1213 = 8u32;
                        fixed_u32::bit_and(a1212, a1213)
                    };
                    let a1215 = 0u32;
                    if equal(a1214, a1215) {
                        let a1218 = {
                            let a1216 = v0;
                            let a1217 = 4u32;
                            fixed_u32::bit_and(a1216, a1217)
                        };
                        let a1219 = 0u32;
                        if equal(a1218, a1219) {
                            let a1222 = {
                                let a1220 = v0;
                                let a1221 = 2u32;
                                fixed_u32::bit_and(a1220, a1221)
                            };
                            let a1223 = 0u32;
                            if equal(a1222, a1223) {
                                let a1226 = {
                                    let a1224 = v0;
                                    let a1225 = 1u32;
                                    fixed_u32::bit_and(a1224, a1225)
                                };
                                let a1227 = 0u32;
                                if equal(a1226, a1227) {
                                    Bytes::lit(&[0xc0])
                                } else {
                                    Bytes::lit(&[0xc1])
                                }
                            } else {
                                let a1230 = {
                                    let a1228 = v0;
                                    let a1229 = 1u32;
                                    fixed_u32::bit_and(a1228, a1229)
                                };
                                let a1231 = 0u32;
                                if equal(a1230, a1231) {
                                    Bytes::lit(&[0xc2])
                                } else {
                                    Bytes::lit(&[0xc3])
                                }
                            }
                        } else {
                            let a1234 = {
                                let a1232 = v0;
                                let a1233 = 2u32;
                                fixed_u32::bit_and(a1232, a1233)
                            };
                            let a1235 = 0u32;
                            if equal(a1234, a1235) {
                                let a1238 = {
                                    let a1236 = v0;
                                    let a1237 = 1u32;
                                    fixed_u32::bit_and(a1236, a1237)
                                };
                                let a1239 = 0u32;
                                if equal(a1238, a1239) {
                                    Bytes::lit(&[0xc4])
                                } else {
                                    Bytes::lit(&[0xc5])
                                }
                            } else {
                                let a1242 = {
                                    let a1240 = v0;
                                    let a1241 = 1u32;
                                    fixed_u32::bit_and(a1240, a1241)
                                };
                                let a1243 = 0u32;
                                if equal(a1242, a1243) {
                                    Bytes::lit(&[0xc6])
                                } else {
                                    Bytes::lit(&[0xc7])
                                }
                            }
                        }
                    } else {
                        let a1246 = {
                            let a1244 = v0;
                            let a1245 = 4u32;
                            fixed_u32::bit_and(a1244, a1245)
                        };
                        let a1247 = 0u32;
                        if equal(a1246, a1247) {
                            let a1250 = {
                                let a1248 = v0;
                                let a1249 = 2u32;
                                fixed_u32::bit_and(a1248, a1249)
                            };
                            let a1251 = 0u32;
                            if equal(a1250, a1251) {
                                let a1254 = {
                                    let a1252 = v0;
                                    let a1253 = 1u32;
                                    fixed_u32::bit_and(a1252, a1253)
                                };
                                let a1255 = 0u32;
                                if equal(a1254, a1255) {
                                    Bytes::lit(&[0xc8])
                                } else {
                                    Bytes::lit(&[0xc9])
                                }
                            } else {
                                let a1258 = {
                                    let a1256 = v0;
                                    let a1257 = 1u32;
                                    fixed_u32::bit_and(a1256, a1257)
                                };
                                let a1259 = 0u32;
                                if equal(a1258, a1259) {
                                    Bytes::lit(&[0xca])
                                } else {
                                    Bytes::lit(&[0xcb])
                                }
                            }
                        } else {
                            let a1262 = {
                                let a1260 = v0;
                                let a1261 = 2u32;
                                fixed_u32::bit_and(a1260, a1261)
                            };
                            let a1263 = 0u32;
                            if equal(a1262, a1263) {
                                let a1266 = {
                                    let a1264 = v0;
                                    let a1265 = 1u32;
                                    fixed_u32::bit_and(a1264, a1265)
                                };
                                let a1267 = 0u32;
                                if equal(a1266, a1267) {
                                    Bytes::lit(&[0xcc])
                                } else {
                                    Bytes::lit(&[0xcd])
                                }
                            } else {
                                let a1270 = {
                                    let a1268 = v0;
                                    let a1269 = 1u32;
                                    fixed_u32::bit_and(a1268, a1269)
                                };
                                let a1271 = 0u32;
                                if equal(a1270, a1271) {
                                    Bytes::lit(&[0xce])
                                } else {
                                    Bytes::lit(&[0xcf])
                                }
                            }
                        }
                    }
                } else {
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
                                    Bytes::lit(&[0xd0])
                                } else {
                                    Bytes::lit(&[0xd1])
                                }
                            } else {
                                let a1290 = {
                                    let a1288 = v0;
                                    let a1289 = 1u32;
                                    fixed_u32::bit_and(a1288, a1289)
                                };
                                let a1291 = 0u32;
                                if equal(a1290, a1291) {
                                    Bytes::lit(&[0xd2])
                                } else {
                                    Bytes::lit(&[0xd3])
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
                                    Bytes::lit(&[0xd4])
                                } else {
                                    Bytes::lit(&[0xd5])
                                }
                            } else {
                                let a1302 = {
                                    let a1300 = v0;
                                    let a1301 = 1u32;
                                    fixed_u32::bit_and(a1300, a1301)
                                };
                                let a1303 = 0u32;
                                if equal(a1302, a1303) {
                                    Bytes::lit(&[0xd6])
                                } else {
                                    Bytes::lit(&[0xd7])
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
                                    Bytes::lit(&[0xd8])
                                } else {
                                    Bytes::lit(&[0xd9])
                                }
                            } else {
                                let a1318 = {
                                    let a1316 = v0;
                                    let a1317 = 1u32;
                                    fixed_u32::bit_and(a1316, a1317)
                                };
                                let a1319 = 0u32;
                                if equal(a1318, a1319) {
                                    Bytes::lit(&[0xda])
                                } else {
                                    Bytes::lit(&[0xdb])
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
                                    Bytes::lit(&[0xdc])
                                } else {
                                    Bytes::lit(&[0xdd])
                                }
                            } else {
                                let a1330 = {
                                    let a1328 = v0;
                                    let a1329 = 1u32;
                                    fixed_u32::bit_and(a1328, a1329)
                                };
                                let a1331 = 0u32;
                                if equal(a1330, a1331) {
                                    Bytes::lit(&[0xde])
                                } else {
                                    Bytes::lit(&[0xdf])
                                }
                            }
                        }
                    }
                }
            } else {
                let a1334 = {
                    let a1332 = v0;
                    let a1333 = 16u32;
                    fixed_u32::bit_and(a1332, a1333)
                };
                let a1335 = 0u32;
                if equal(a1334, a1335) {
                    let a1338 = {
                        let a1336 = v0;
                        let a1337 = 8u32;
                        fixed_u32::bit_and(a1336, a1337)
                    };
                    let a1339 = 0u32;
                    if equal(a1338, a1339) {
                        let a1342 = {
                            let a1340 = v0;
                            let a1341 = 4u32;
                            fixed_u32::bit_and(a1340, a1341)
                        };
                        let a1343 = 0u32;
                        if equal(a1342, a1343) {
                            let a1346 = {
                                let a1344 = v0;
                                let a1345 = 2u32;
                                fixed_u32::bit_and(a1344, a1345)
                            };
                            let a1347 = 0u32;
                            if equal(a1346, a1347) {
                                let a1350 = {
                                    let a1348 = v0;
                                    let a1349 = 1u32;
                                    fixed_u32::bit_and(a1348, a1349)
                                };
                                let a1351 = 0u32;
                                if equal(a1350, a1351) {
                                    Bytes::lit(&[0xe0])
                                } else {
                                    Bytes::lit(&[0xe1])
                                }
                            } else {
                                let a1354 = {
                                    let a1352 = v0;
                                    let a1353 = 1u32;
                                    fixed_u32::bit_and(a1352, a1353)
                                };
                                let a1355 = 0u32;
                                if equal(a1354, a1355) {
                                    Bytes::lit(&[0xe2])
                                } else {
                                    Bytes::lit(&[0xe3])
                                }
                            }
                        } else {
                            let a1358 = {
                                let a1356 = v0;
                                let a1357 = 2u32;
                                fixed_u32::bit_and(a1356, a1357)
                            };
                            let a1359 = 0u32;
                            if equal(a1358, a1359) {
                                let a1362 = {
                                    let a1360 = v0;
                                    let a1361 = 1u32;
                                    fixed_u32::bit_and(a1360, a1361)
                                };
                                let a1363 = 0u32;
                                if equal(a1362, a1363) {
                                    Bytes::lit(&[0xe4])
                                } else {
                                    Bytes::lit(&[0xe5])
                                }
                            } else {
                                let a1366 = {
                                    let a1364 = v0;
                                    let a1365 = 1u32;
                                    fixed_u32::bit_and(a1364, a1365)
                                };
                                let a1367 = 0u32;
                                if equal(a1366, a1367) {
                                    Bytes::lit(&[0xe6])
                                } else {
                                    Bytes::lit(&[0xe7])
                                }
                            }
                        }
                    } else {
                        let a1370 = {
                            let a1368 = v0;
                            let a1369 = 4u32;
                            fixed_u32::bit_and(a1368, a1369)
                        };
                        let a1371 = 0u32;
                        if equal(a1370, a1371) {
                            let a1374 = {
                                let a1372 = v0;
                                let a1373 = 2u32;
                                fixed_u32::bit_and(a1372, a1373)
                            };
                            let a1375 = 0u32;
                            if equal(a1374, a1375) {
                                let a1378 = {
                                    let a1376 = v0;
                                    let a1377 = 1u32;
                                    fixed_u32::bit_and(a1376, a1377)
                                };
                                let a1379 = 0u32;
                                if equal(a1378, a1379) {
                                    Bytes::lit(&[0xe8])
                                } else {
                                    Bytes::lit(&[0xe9])
                                }
                            } else {
                                let a1382 = {
                                    let a1380 = v0;
                                    let a1381 = 1u32;
                                    fixed_u32::bit_and(a1380, a1381)
                                };
                                let a1383 = 0u32;
                                if equal(a1382, a1383) {
                                    Bytes::lit(&[0xea])
                                } else {
                                    Bytes::lit(&[0xeb])
                                }
                            }
                        } else {
                            let a1386 = {
                                let a1384 = v0;
                                let a1385 = 2u32;
                                fixed_u32::bit_and(a1384, a1385)
                            };
                            let a1387 = 0u32;
                            if equal(a1386, a1387) {
                                let a1390 = {
                                    let a1388 = v0;
                                    let a1389 = 1u32;
                                    fixed_u32::bit_and(a1388, a1389)
                                };
                                let a1391 = 0u32;
                                if equal(a1390, a1391) {
                                    Bytes::lit(&[0xec])
                                } else {
                                    Bytes::lit(&[0xed])
                                }
                            } else {
                                let a1394 = {
                                    let a1392 = v0;
                                    let a1393 = 1u32;
                                    fixed_u32::bit_and(a1392, a1393)
                                };
                                let a1395 = 0u32;
                                if equal(a1394, a1395) {
                                    Bytes::lit(&[0xee])
                                } else {
                                    Bytes::lit(&[0xef])
                                }
                            }
                        }
                    }
                } else {
                    let a1398 = {
                        let a1396 = v0;
                        let a1397 = 8u32;
                        fixed_u32::bit_and(a1396, a1397)
                    };
                    let a1399 = 0u32;
                    if equal(a1398, a1399) {
                        let a1402 = {
                            let a1400 = v0;
                            let a1401 = 4u32;
                            fixed_u32::bit_and(a1400, a1401)
                        };
                        let a1403 = 0u32;
                        if equal(a1402, a1403) {
                            let a1406 = {
                                let a1404 = v0;
                                let a1405 = 2u32;
                                fixed_u32::bit_and(a1404, a1405)
                            };
                            let a1407 = 0u32;
                            if equal(a1406, a1407) {
                                let a1410 = {
                                    let a1408 = v0;
                                    let a1409 = 1u32;
                                    fixed_u32::bit_and(a1408, a1409)
                                };
                                let a1411 = 0u32;
                                if equal(a1410, a1411) {
                                    Bytes::lit(&[0xf0])
                                } else {
                                    Bytes::lit(&[0xf1])
                                }
                            } else {
                                let a1414 = {
                                    let a1412 = v0;
                                    let a1413 = 1u32;
                                    fixed_u32::bit_and(a1412, a1413)
                                };
                                let a1415 = 0u32;
                                if equal(a1414, a1415) {
                                    Bytes::lit(&[0xf2])
                                } else {
                                    Bytes::lit(&[0xf3])
                                }
                            }
                        } else {
                            let a1418 = {
                                let a1416 = v0;
                                let a1417 = 2u32;
                                fixed_u32::bit_and(a1416, a1417)
                            };
                            let a1419 = 0u32;
                            if equal(a1418, a1419) {
                                let a1422 = {
                                    let a1420 = v0;
                                    let a1421 = 1u32;
                                    fixed_u32::bit_and(a1420, a1421)
                                };
                                let a1423 = 0u32;
                                if equal(a1422, a1423) {
                                    Bytes::lit(&[0xf4])
                                } else {
                                    Bytes::lit(&[0xf5])
                                }
                            } else {
                                let a1426 = {
                                    let a1424 = v0;
                                    let a1425 = 1u32;
                                    fixed_u32::bit_and(a1424, a1425)
                                };
                                let a1427 = 0u32;
                                if equal(a1426, a1427) {
                                    Bytes::lit(&[0xf6])
                                } else {
                                    Bytes::lit(&[0xf7])
                                }
                            }
                        }
                    } else {
                        let a1430 = {
                            let a1428 = v0;
                            let a1429 = 4u32;
                            fixed_u32::bit_and(a1428, a1429)
                        };
                        let a1431 = 0u32;
                        if equal(a1430, a1431) {
                            let a1434 = {
                                let a1432 = v0;
                                let a1433 = 2u32;
                                fixed_u32::bit_and(a1432, a1433)
                            };
                            let a1435 = 0u32;
                            if equal(a1434, a1435) {
                                let a1438 = {
                                    let a1436 = v0;
                                    let a1437 = 1u32;
                                    fixed_u32::bit_and(a1436, a1437)
                                };
                                let a1439 = 0u32;
                                if equal(a1438, a1439) {
                                    Bytes::lit(&[0xf8])
                                } else {
                                    Bytes::lit(&[0xf9])
                                }
                            } else {
                                let a1442 = {
                                    let a1440 = v0;
                                    let a1441 = 1u32;
                                    fixed_u32::bit_and(a1440, a1441)
                                };
                                let a1443 = 0u32;
                                if equal(a1442, a1443) {
                                    Bytes::lit(&[0xfa])
                                } else {
                                    Bytes::lit(&[0xfb])
                                }
                            }
                        } else {
                            let a1446 = {
                                let a1444 = v0;
                                let a1445 = 2u32;
                                fixed_u32::bit_and(a1444, a1445)
                            };
                            let a1447 = 0u32;
                            if equal(a1446, a1447) {
                                let a1450 = {
                                    let a1448 = v0;
                                    let a1449 = 1u32;
                                    fixed_u32::bit_and(a1448, a1449)
                                };
                                let a1451 = 0u32;
                                if equal(a1450, a1451) {
                                    Bytes::lit(&[0xfc])
                                } else {
                                    Bytes::lit(&[0xfd])
                                }
                            } else {
                                let a1454 = {
                                    let a1452 = v0;
                                    let a1453 = 1u32;
                                    fixed_u32::bit_and(a1452, a1453)
                                };
                                let a1455 = 0u32;
                                if equal(a1454, a1455) {
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

pub fn f61(v0: u32, v1: u32) -> u32 {
    let m1458 = {
        let a1456 = v0;
        let a1457 = v1;
        fixed_u32::shift_right(a1456, a1457)
    };
    match m1458 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f62(v0: Fn10, p1462: (Option<Bytes>, List<Bytes>), p1463: List<Bytes>) -> (Option<Bytes>, List<Bytes>) {
    let m1461 = (p1462, p1463);
    {
        let mut m1461 = m1461;
        loop {
            let step = {
                let (v1, v2) = m1461;
                let m1459 = v2;
                match m1459.uncons() {
                    None => {
                        Err::<((Option<Bytes>, List<Bytes>), List<Bytes>), (Option<Bytes>, List<Bytes>)>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<((Option<Bytes>, List<Bytes>), List<Bytes>), (Option<Bytes>, List<Bytes>)>(({
                            let c1460: Fn10 = v0.clone();
                            c1460.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m1461 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f63(v0: (Option<Bytes>, List<Bytes>), v1: Bytes) -> (Option<Bytes>, List<Bytes>) {
    let m1465 = {
        let (h1464, _) = v0;
        h1464
    };
    match m1465 {
        None => {
            (Some(v1), {
                let (_, h1469) = v0;
                h1469
            })
        }
        Some(v2) => {
            (None::<Bytes>, List::cons({
                let a1466 = v2;
                let a1467 = v1;
                append_bytes(a1466, a1467)
            }, {
                let (_, h1468) = v0;
                h1468
            }))
        }
    }
}

pub fn f64(v0: List<Bytes>, v1: Bytes) -> List<Bytes> {
    List::cons(v1, v0)
}

pub fn f65(v0: u32, v1: u32, v2: u32, v3: u32) -> u32 {
    let a1474 = v0;
    let a1475 = {
        let a1472 = f73(v1, 8u32);
        let a1473 = {
            let a1470 = f73(v2, 16u32);
            let a1471 = f73(v3, 24u32);
            fixed_u32::bit_or(a1470, a1471)
        };
        fixed_u32::bit_or(a1472, a1473)
    };
    fixed_u32::bit_or(a1474, a1475)
}

pub fn f66(v0: Bytes, v1: u64, v2: u64) -> u32 {
    let a1476 = v2;
    let a1477 = v1;
    if nat_lt(a1476, a1477) {
        let m1480 = {
            let a1478 = v0;
            let a1479 = v2;
            index_bytes(a1478, a1479)
        };
        match m1480 {
            None => {
                0u32
            }
            Some(v3) => {
                let m1482 = {
                    let a1481 = v3;
                    fixed_u32::convert(i128::from(a1481))
                };
                match m1482 {
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

pub fn f67(v0: Fn11, p1486: u64, p1487: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let m1485 = (p1486, p1487);
    {
        let mut m1485 = m1485;
        loop {
            let step = {
                let (v1, v2) = m1485;
                let m1483 = v1;
                if m1483 == 0 {
                    Err::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>(v2)
                } else {
                    let v3 = m1483 - 1;
                    Ok::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>((v3, {
                        let c1484: Fn11 = v0.clone();
                        c1484.apply(v2)
                    }))
                }
            };
            match step {
                Ok(next) => { m1485 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f68(v0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let v1: u64 = {
        let (h1488, _) = v0;
        h1488
    };
    let v2: u32 = {
        let (h1489, _) = {
            let (_, h1490) = v0;
            h1490
        };
        h1489
    };
    let v3: u32 = {
        let (_, h1491) = {
            let (_, h1492) = v0;
            h1492
        };
        h1491
    };
    ({
        let a1493 = v1;
        let a1494 = 2u64;
        let a1495 = 0u64;
        nat_quot(a1493, a1494, a1495)
    }, (f73(v2, 1u32), {
        let a1499 = {
            let a1496 = v1;
            let a1497 = 2u64;
            let a1498 = 0u64;
            nat_rem(a1496, a1497, a1498)
        };
        let a1500 = 1u64;
        if nat_eq(a1499, a1500) {
            let a1501 = v3;
            let a1502 = v2;
            fixed_u32::bit_or(a1501, a1502)
        } else {
            v3
        }
    }))
}

pub fn f69(v0: Adt11) -> Adt6 {
    f74(Fn13::F75, Adt6::C0(0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32), match v0 {
        Adt11::C0(_, _, _, _, _, _, h1503) => {
            h1503
        }
    })
}

pub fn f70(v0: Adt3, v1: Adt6, v2: u32, v3: u32, v4: u32, v5: u32) -> R<Adt11> {
    let v6: (u64, Adt14) = f76(Fn14::F77(v0.clone(), v1.clone()), 800u64, (0u64, Adt14::C0(f78(v0.clone(), v2, v3, v4, v5), Adt6::C0(0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32), List::<Adt12>::nil())))?;
    Ok(Adt11::C0(v0, v1, v2, v3, v4, v5, {
        let m1506 = {
            let (_, h1505) = v6;
            h1505
        };
        match m1506 {
            Adt14::C0(_, _, h1504) => {
                h1504
            }
        }
    }))
}

pub fn f71(v0: Fn12, p1511: u64, p1512: Adt10) -> R<(Adt10, bool)> {
    let m1510 = (p1511, p1512);
    {
        let mut m1510 = m1510;
        loop {
            let step = {
                let (v1, v2) = m1510;
                let m1507 = v1;
                if m1507 == 0 {
                    Err::<(u64, Adt10), R<(Adt10, bool)>>(Ok((v2, false)))
                } else {
                    let v3 = m1507 - 1;
                    let m1509 = {
                        let c1508: Fn12 = v0.clone();
                        c1508.apply(v2.clone())?
                    };
                    match m1509 {
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
                Ok(next) => { m1510 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f72(v0: u32, v1: Adt3, v2: Adt10) -> R<Option<Adt10>> {
    f79(v1, v0, v2)
}

pub fn f73(v0: u32, v1: u32) -> u32 {
    let m1515 = {
        let a1513 = v0;
        let a1514 = v1;
        fixed_u32::shift_left(a1513, a1514)
    };
    match m1515 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f74(v0: Fn13, p1519: Adt6, p1520: List<Adt12>) -> Adt6 {
    let m1518 = (p1519, p1520);
    {
        let mut m1518 = m1518;
        loop {
            let step = {
                let (v1, v2) = m1518;
                let m1516 = v2;
                match m1516.uncons() {
                    None => {
                        Err::<(Adt6, List<Adt12>), Adt6>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt6, List<Adt12>), Adt6>(({
                            let c1517: Fn13 = v0.clone();
                            c1517.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m1518 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f75(v0: Adt6, v1: Adt12) -> Adt6 {
    let a1522 = match v1.clone() {
        Adt12::C0(h1521, _, _, _, _, _, _, _, _) => {
            h1521
        }
    };
    let a1523 = 7u64;
    if nat_eq(a1522, a1523) {
        f80(v0, match v1.clone() {
            Adt12::C0(_, _, _, _, h1524, _, _, _, _) => {
                h1524
            }
        }, match v1 {
            Adt12::C0(_, _, _, _, _, _, _, _, h1525) => {
                h1525
            }
        })
    } else {
        v0
    }
}

pub fn f76(v0: Fn14, p1529: u64, p1530: (u64, Adt14)) -> R<(u64, Adt14)> {
    let m1528 = (p1529, p1530);
    {
        let mut m1528 = m1528;
        loop {
            let step = {
                let (v1, v2) = m1528;
                let m1526 = v1;
                if m1526 == 0 {
                    Err::<(u64, (u64, Adt14)), R<(u64, Adt14)>>(Ok(v2))
                } else {
                    let v3 = m1526 - 1;
                    Ok::<(u64, (u64, Adt14)), R<(u64, Adt14)>>((v3, {
                        let c1527: Fn14 = v0.clone();
                        c1527.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m1528 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f77(v0: Adt3, v1: Adt6, v2: (u64, Adt14)) -> R<(u64, Adt14)> {
    Ok(({
        let a1532 = {
            let (h1531, _) = v2.clone();
            h1531
        };
        let a1533 = 1u64;
        nat_add(a1532, a1533)?
    }, f81(v0, v1, {
        let (_, h1534) = v2;
        h1534
    }, {
        let (h1535, _) = v2;
        h1535
    })?))
}

pub fn f78(v0: Adt3, v1: u32, v2: u32, v3: u32, v4: u32) -> Adt6 {
    Adt6::C0(match v0.clone() {
        Adt3::C0(h1536, _, _, _, _, _, _, _) => {
            h1536
        }
    }, match v0.clone() {
        Adt3::C0(_, h1537, _, _, _, _, _, _) => {
            h1537
        }
    }, match v0.clone() {
        Adt3::C0(_, _, h1538, _, _, _, _, _) => {
            h1538
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h1539, _, _, _, _) => {
            h1539
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h1540, _, _, _) => {
            h1540
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h1541, _, _) => {
            h1541
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h1542, _) => {
            h1542
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, h1543) => {
            h1543
        }
    }, 1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, v1, v2, v3, v4)
}

pub fn f79(v0: Adt3, v1: u32, v2: Adt10) -> R<Option<Adt10>> {
    let a1548 = {
        let a1545 = match v2.clone() {
            Adt10::C0(_, h1544, _, _) => {
                h1544
            }
        };
        let a1546 = 2u64;
        let a1547 = 0u64;
        nat_rem(a1545, a1546, a1547)
    };
    let a1549 = 0u64;
    if nat_eq(a1548, a1549) {
        let m1551 = match v2.clone() {
            Adt10::C0(_, _, h1550, _) => {
                h1550
            }
        };
        match m1551.uncons() {
            None => {
                Ok(None::<Adt10>)
            }
            Some((v3, v4)) => {
                let v5: Adt5 = f39(f40(v3.clone(), match v2.clone() {
                    Adt10::C0(h1552, _, _, _) => {
                        h1552
                    }
                }, v0, v1)?)?;
                Ok(Some(Adt10::C0(Adt7::C0(f28(match v5.clone() {
                    Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h1553) => {
                        h1553
                    }
                }), match v3.clone() {
                    Adt7::C0(_, h1554, _) => {
                        h1554
                    }
                }, {
                    let a1559 = match v3.clone() {
                        Adt7::C0(_, _, h1555) => {
                            h1555
                        }
                    };
                    let a1560 = {
                        let m1558 = match v2.clone() {
                            Adt10::C0(h1557, _, _, _) => {
                                h1557
                            }
                        };
                        match m1558 {
                            Adt7::C0(_, _, h1556) => {
                                h1556
                            }
                        }
                    };
                    nat_add(a1559, a1560)?
                }), {
                    let a1562 = match v2.clone() {
                        Adt10::C0(_, h1561, _, _) => {
                            h1561
                        }
                    };
                    let a1563 = 2u64;
                    let a1564 = 0u64;
                    nat_quot(a1562, a1563, a1564)
                }, v4, List::cons(Adt4::C0(v5), List::cons(Adt4::C2(v3, {
                    let a1566 = match v2.clone() {
                        Adt10::C0(_, _, h1565, _) => {
                            h1565
                        }
                    };
                    length_list(a1566)
                }), match v2 {
                    Adt10::C0(_, _, _, h1567) => {
                        h1567
                    }
                })))))
            }
        }
    } else {
        Ok(None::<Adt10>)
    }
}

pub fn f80(v0: Adt6, v1: u64, v2: u32) -> Adt6 {
    let a1568 = v1;
    let a1569 = 8u64;
    if nat_lt(a1568, a1569) {
        let a1570 = v1;
        let a1571 = 4u64;
        if nat_lt(a1570, a1571) {
            let a1572 = v1;
            let a1573 = 2u64;
            if nat_lt(a1572, a1573) {
                let a1574 = v1;
                let a1575 = 1u64;
                if nat_lt(a1574, a1575) {
                    Adt6::C0(v2, match v0.clone() {
                        Adt6::C0(_, h1576, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1576
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h1577, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1577
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h1578, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1578
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h1579, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1579
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h1580, _, _, _, _, _, _, _, _, _, _) => {
                            h1580
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h1581, _, _, _, _, _, _, _, _, _) => {
                            h1581
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h1582, _, _, _, _, _, _, _, _) => {
                            h1582
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h1583, _, _, _, _, _, _, _) => {
                            h1583
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h1584, _, _, _, _, _, _) => {
                            h1584
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h1585, _, _, _, _, _) => {
                            h1585
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h1586, _, _, _, _) => {
                            h1586
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1587, _, _, _) => {
                            h1587
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1588, _, _) => {
                            h1588
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1589, _) => {
                            h1589
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1590) => {
                            h1590
                        }
                    })
                } else {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h1591, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1591
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, h1592, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1592
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h1593, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1593
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h1594, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1594
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h1595, _, _, _, _, _, _, _, _, _, _) => {
                            h1595
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h1596, _, _, _, _, _, _, _, _, _) => {
                            h1596
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h1597, _, _, _, _, _, _, _, _) => {
                            h1597
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h1598, _, _, _, _, _, _, _) => {
                            h1598
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h1599, _, _, _, _, _, _) => {
                            h1599
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h1600, _, _, _, _, _) => {
                            h1600
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h1601, _, _, _, _) => {
                            h1601
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1602, _, _, _) => {
                            h1602
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1603, _, _) => {
                            h1603
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1604, _) => {
                            h1604
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1605) => {
                            h1605
                        }
                    })
                }
            } else {
                let a1606 = v1;
                let a1607 = 3u64;
                if nat_lt(a1606, a1607) {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h1608, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1608
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h1609, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1609
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, h1610, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1610
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h1611, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1611
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h1612, _, _, _, _, _, _, _, _, _, _) => {
                            h1612
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h1613, _, _, _, _, _, _, _, _, _) => {
                            h1613
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h1614, _, _, _, _, _, _, _, _) => {
                            h1614
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h1615, _, _, _, _, _, _, _) => {
                            h1615
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h1616, _, _, _, _, _, _) => {
                            h1616
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h1617, _, _, _, _, _) => {
                            h1617
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h1618, _, _, _, _) => {
                            h1618
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1619, _, _, _) => {
                            h1619
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1620, _, _) => {
                            h1620
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1621, _) => {
                            h1621
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1622) => {
                            h1622
                        }
                    })
                } else {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h1623, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1623
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h1624, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1624
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h1625, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1625
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, h1626, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1626
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h1627, _, _, _, _, _, _, _, _, _, _) => {
                            h1627
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h1628, _, _, _, _, _, _, _, _, _) => {
                            h1628
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h1629, _, _, _, _, _, _, _, _) => {
                            h1629
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h1630, _, _, _, _, _, _, _) => {
                            h1630
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h1631, _, _, _, _, _, _) => {
                            h1631
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h1632, _, _, _, _, _) => {
                            h1632
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h1633, _, _, _, _) => {
                            h1633
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1634, _, _, _) => {
                            h1634
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1635, _, _) => {
                            h1635
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1636, _) => {
                            h1636
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1637) => {
                            h1637
                        }
                    })
                }
            }
        } else {
            let a1638 = v1;
            let a1639 = 6u64;
            if nat_lt(a1638, a1639) {
                let a1640 = v1;
                let a1641 = 5u64;
                if nat_lt(a1640, a1641) {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h1642, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1642
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h1643, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1643
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h1644, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1644
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h1645, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1645
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h1646, _, _, _, _, _, _, _, _, _, _) => {
                            h1646
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h1647, _, _, _, _, _, _, _, _, _) => {
                            h1647
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h1648, _, _, _, _, _, _, _, _) => {
                            h1648
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h1649, _, _, _, _, _, _, _) => {
                            h1649
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h1650, _, _, _, _, _, _) => {
                            h1650
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h1651, _, _, _, _, _) => {
                            h1651
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h1652, _, _, _, _) => {
                            h1652
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1653, _, _, _) => {
                            h1653
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1654, _, _) => {
                            h1654
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1655, _) => {
                            h1655
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1656) => {
                            h1656
                        }
                    })
                } else {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h1657, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1657
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h1658, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1658
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h1659, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1659
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h1660, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1660
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h1661, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1661
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h1662, _, _, _, _, _, _, _, _, _) => {
                            h1662
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h1663, _, _, _, _, _, _, _, _) => {
                            h1663
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h1664, _, _, _, _, _, _, _) => {
                            h1664
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h1665, _, _, _, _, _, _) => {
                            h1665
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h1666, _, _, _, _, _) => {
                            h1666
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h1667, _, _, _, _) => {
                            h1667
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1668, _, _, _) => {
                            h1668
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1669, _, _) => {
                            h1669
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1670, _) => {
                            h1670
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1671) => {
                            h1671
                        }
                    })
                }
            } else {
                let a1672 = v1;
                let a1673 = 7u64;
                if nat_lt(a1672, a1673) {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h1674, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1674
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h1675, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1675
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h1676, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1676
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h1677, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1677
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h1678, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1678
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h1679, _, _, _, _, _, _, _, _, _, _) => {
                            h1679
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h1680, _, _, _, _, _, _, _, _) => {
                            h1680
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h1681, _, _, _, _, _, _, _) => {
                            h1681
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h1682, _, _, _, _, _, _) => {
                            h1682
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h1683, _, _, _, _, _) => {
                            h1683
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h1684, _, _, _, _) => {
                            h1684
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1685, _, _, _) => {
                            h1685
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1686, _, _) => {
                            h1686
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1687, _) => {
                            h1687
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1688) => {
                            h1688
                        }
                    })
                } else {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h1689, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1689
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h1690, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1690
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h1691, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1691
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h1692, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1692
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h1693, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1693
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h1694, _, _, _, _, _, _, _, _, _, _) => {
                            h1694
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h1695, _, _, _, _, _, _, _, _, _) => {
                            h1695
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h1696, _, _, _, _, _, _, _) => {
                            h1696
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h1697, _, _, _, _, _, _) => {
                            h1697
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h1698, _, _, _, _, _) => {
                            h1698
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h1699, _, _, _, _) => {
                            h1699
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1700, _, _, _) => {
                            h1700
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1701, _, _) => {
                            h1701
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1702, _) => {
                            h1702
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1703) => {
                            h1703
                        }
                    })
                }
            }
        }
    } else {
        let a1704 = v1;
        let a1705 = 12u64;
        if nat_lt(a1704, a1705) {
            let a1706 = v1;
            let a1707 = 10u64;
            if nat_lt(a1706, a1707) {
                let a1708 = v1;
                let a1709 = 9u64;
                if nat_lt(a1708, a1709) {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h1710, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1710
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h1711, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1711
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h1712, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1712
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h1713, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1713
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h1714, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1714
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h1715, _, _, _, _, _, _, _, _, _, _) => {
                            h1715
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h1716, _, _, _, _, _, _, _, _, _) => {
                            h1716
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h1717, _, _, _, _, _, _, _, _) => {
                            h1717
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h1718, _, _, _, _, _, _) => {
                            h1718
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h1719, _, _, _, _, _) => {
                            h1719
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h1720, _, _, _, _) => {
                            h1720
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1721, _, _, _) => {
                            h1721
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1722, _, _) => {
                            h1722
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1723, _) => {
                            h1723
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1724) => {
                            h1724
                        }
                    })
                } else {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h1725, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1725
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h1726, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1726
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h1727, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1727
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h1728, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1728
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h1729, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1729
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h1730, _, _, _, _, _, _, _, _, _, _) => {
                            h1730
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h1731, _, _, _, _, _, _, _, _, _) => {
                            h1731
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h1732, _, _, _, _, _, _, _, _) => {
                            h1732
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h1733, _, _, _, _, _, _, _) => {
                            h1733
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h1734, _, _, _, _, _) => {
                            h1734
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h1735, _, _, _, _) => {
                            h1735
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1736, _, _, _) => {
                            h1736
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1737, _, _) => {
                            h1737
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1738, _) => {
                            h1738
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1739) => {
                            h1739
                        }
                    })
                }
            } else {
                let a1740 = v1;
                let a1741 = 11u64;
                if nat_lt(a1740, a1741) {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h1742, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1742
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h1743, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1743
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h1744, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1744
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h1745, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1745
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h1746, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1746
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h1747, _, _, _, _, _, _, _, _, _, _) => {
                            h1747
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h1748, _, _, _, _, _, _, _, _, _) => {
                            h1748
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h1749, _, _, _, _, _, _, _, _) => {
                            h1749
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h1750, _, _, _, _, _, _, _) => {
                            h1750
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h1751, _, _, _, _, _, _) => {
                            h1751
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h1752, _, _, _, _) => {
                            h1752
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1753, _, _, _) => {
                            h1753
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1754, _, _) => {
                            h1754
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1755, _) => {
                            h1755
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1756) => {
                            h1756
                        }
                    })
                } else {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h1757, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1757
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h1758, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1758
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h1759, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1759
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h1760, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1760
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h1761, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1761
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h1762, _, _, _, _, _, _, _, _, _, _) => {
                            h1762
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h1763, _, _, _, _, _, _, _, _, _) => {
                            h1763
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h1764, _, _, _, _, _, _, _, _) => {
                            h1764
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h1765, _, _, _, _, _, _, _) => {
                            h1765
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h1766, _, _, _, _, _, _) => {
                            h1766
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h1767, _, _, _, _, _) => {
                            h1767
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1768, _, _, _) => {
                            h1768
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1769, _, _) => {
                            h1769
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1770, _) => {
                            h1770
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1771) => {
                            h1771
                        }
                    })
                }
            }
        } else {
            let a1772 = v1;
            let a1773 = 14u64;
            if nat_lt(a1772, a1773) {
                let a1774 = v1;
                let a1775 = 13u64;
                if nat_lt(a1774, a1775) {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h1776, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1776
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h1777, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1777
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h1778, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1778
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h1779, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1779
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h1780, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1780
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h1781, _, _, _, _, _, _, _, _, _, _) => {
                            h1781
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h1782, _, _, _, _, _, _, _, _, _) => {
                            h1782
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h1783, _, _, _, _, _, _, _, _) => {
                            h1783
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h1784, _, _, _, _, _, _, _) => {
                            h1784
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h1785, _, _, _, _, _, _) => {
                            h1785
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h1786, _, _, _, _, _) => {
                            h1786
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h1787, _, _, _, _) => {
                            h1787
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1788, _, _) => {
                            h1788
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1789, _) => {
                            h1789
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1790) => {
                            h1790
                        }
                    })
                } else {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h1791, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1791
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h1792, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1792
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h1793, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1793
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h1794, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1794
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h1795, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1795
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h1796, _, _, _, _, _, _, _, _, _, _) => {
                            h1796
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h1797, _, _, _, _, _, _, _, _, _) => {
                            h1797
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h1798, _, _, _, _, _, _, _, _) => {
                            h1798
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h1799, _, _, _, _, _, _, _) => {
                            h1799
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h1800, _, _, _, _, _, _) => {
                            h1800
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h1801, _, _, _, _, _) => {
                            h1801
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h1802, _, _, _, _) => {
                            h1802
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1803, _, _, _) => {
                            h1803
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1804, _) => {
                            h1804
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1805) => {
                            h1805
                        }
                    })
                }
            } else {
                let a1806 = v1;
                let a1807 = 15u64;
                if nat_lt(a1806, a1807) {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h1808, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1808
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h1809, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1809
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h1810, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1810
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h1811, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1811
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h1812, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1812
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h1813, _, _, _, _, _, _, _, _, _, _) => {
                            h1813
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h1814, _, _, _, _, _, _, _, _, _) => {
                            h1814
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h1815, _, _, _, _, _, _, _, _) => {
                            h1815
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h1816, _, _, _, _, _, _, _) => {
                            h1816
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h1817, _, _, _, _, _, _) => {
                            h1817
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h1818, _, _, _, _, _) => {
                            h1818
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h1819, _, _, _, _) => {
                            h1819
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1820, _, _, _) => {
                            h1820
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1821, _, _) => {
                            h1821
                        }
                    }, v2, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1822) => {
                            h1822
                        }
                    })
                } else {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h1823, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1823
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h1824, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1824
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h1825, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1825
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h1826, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1826
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h1827, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1827
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h1828, _, _, _, _, _, _, _, _, _, _) => {
                            h1828
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h1829, _, _, _, _, _, _, _, _, _) => {
                            h1829
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h1830, _, _, _, _, _, _, _, _) => {
                            h1830
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h1831, _, _, _, _, _, _, _) => {
                            h1831
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h1832, _, _, _, _, _, _) => {
                            h1832
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h1833, _, _, _, _, _) => {
                            h1833
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h1834, _, _, _, _) => {
                            h1834
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1835, _, _, _) => {
                            h1835
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1836, _, _) => {
                            h1836
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1837, _) => {
                            h1837
                        }
                    }, v2)
                }
            }
        }
    }
}

pub fn f81(v0: Adt3, v1: Adt6, v2: Adt14, v3: u64) -> R<Adt14> {
    let v4: u64 = {
        let a1838 = v3;
        let a1839 = 784u64;
        if nat_lt(a1838, a1839) {
            let a1840 = v3;
            let a1841 = 112u64;
            let a1842 = 0u64;
            nat_quot(a1840, a1841, a1842)
        } else {
            7u64
        }
    };
    let v5: Adt15 = f82({
        let a1843 = v3;
        let a1844 = 784u64;
        if nat_lt(a1843, a1844) {
            let a1845 = v3;
            let a1846 = 112u64;
            let a1847 = 0u64;
            nat_rem(a1845, a1846, a1847)
        } else {
            let a1848 = v3;
            let a1849 = 672u64;
            nat_sub(a1848, a1849)
        }
    });
    let v6: Adt6 = match v2.clone() {
        Adt14::C0(h1850, _, _) => {
            h1850
        }
    };
    let v7: u32 = f83(v6.clone(), match v5.clone() {
        Adt15::C0(_, _, _, h1851, _) => {
            h1851
        }
    });
    let v8: u32 = {
        let m1853 = match v5.clone() {
            Adt15::C0(_, _, h1852, _, _) => {
                h1852
            }
        };
        match m1853 {
            Adt13::C0 => {
                f83(v6.clone(), match v5.clone() {
                    Adt15::C0(_, _, _, _, h1854) => {
                        h1854
                    }
                })
            }
            Adt13::C1 => {
                f83(v1, f84(v4, match v5.clone() {
                    Adt15::C0(_, _, _, _, h1855) => {
                        h1855
                    }
                })?)
            }
            Adt13::C2 => {
                f83(v6.clone(), match v5.clone() {
                    Adt15::C0(_, _, _, _, h1856) => {
                        h1856
                    }
                })
            }
            Adt13::C3 => {
                f85(match v5.clone() {
                    Adt15::C0(_, _, _, _, h1857) => {
                        h1857
                    }
                })
            }
            Adt13::C4 => {
                f86(v0, match v5.clone() {
                    Adt15::C0(_, _, _, _, h1858) => {
                        h1858
                    }
                })
            }
        }
    };
    let v9: u32 = f87(match v5.clone() {
        Adt15::C0(_, _, h1859, _, _) => {
            h1859
        }
    }, v7, v8);
    Ok(Adt14::C0({
        let a1860 = v4;
        let a1861 = 7u64;
        if nat_eq(a1860, a1861) {
            v6
        } else {
            f80(v6, match v5.clone() {
                Adt15::C0(_, _, _, h1862, _) => {
                    h1862
                }
            }, v9)
        }
    }, {
        let a1863 = v4;
        let a1864 = 7u64;
        if nat_eq(a1863, a1864) {
            f80(match v2.clone() {
                Adt14::C0(_, h1865, _) => {
                    h1865
                }
            }, match v5.clone() {
                Adt15::C0(_, _, _, h1866, _) => {
                    h1866
                }
            }, v9)
        } else {
            match v2.clone() {
                Adt14::C0(_, h1867, _) => {
                    h1867
                }
            }
        }
    }, List::cons(Adt12::C0(v4, match v5.clone() {
        Adt15::C0(h1868, _, _, _, _) => {
            h1868
        }
    }, match v5.clone() {
        Adt15::C0(_, h1869, _, _, _) => {
            h1869
        }
    }, match v5.clone() {
        Adt15::C0(_, _, h1870, _, _) => {
            h1870
        }
    }, match v5.clone() {
        Adt15::C0(_, _, _, h1871, _) => {
            h1871
        }
    }, match v5 {
        Adt15::C0(_, _, _, _, h1872) => {
            h1872
        }
    }, v7, v8, v9), match v2 {
        Adt14::C0(_, _, h1873) => {
            h1873
        }
    })))
}

pub fn f82(v0: u64) -> Adt15 {
    let a1874 = v0;
    let a1875 = 64u64;
    if nat_lt(a1874, a1875) {
        let a1876 = v0;
        let a1877 = 32u64;
        if nat_lt(a1876, a1877) {
            let a1878 = v0;
            let a1879 = 16u64;
            if nat_lt(a1878, a1879) {
                let a1880 = v0;
                let a1881 = 8u64;
                if nat_lt(a1880, a1881) {
                    let a1882 = v0;
                    let a1883 = 4u64;
                    if nat_lt(a1882, a1883) {
                        let a1884 = v0;
                        let a1885 = 2u64;
                        if nat_lt(a1884, a1885) {
                            let a1886 = v0;
                            let a1887 = 1u64;
                            if nat_lt(a1886, a1887) {
                                Adt15::C0(0u64, 0u64, Adt13::C0, 0u64, 4u64)
                            } else {
                                Adt15::C0(0u64, 1u64, Adt13::C1, 0u64, 0u64)
                            }
                        } else {
                            let a1888 = v0;
                            let a1889 = 3u64;
                            if nat_lt(a1888, a1889) {
                                Adt15::C0(0u64, 2u64, Adt13::C2, 12u64, 0u64)
                            } else {
                                Adt15::C0(0u64, 3u64, Adt13::C3, 12u64, 16u64)
                            }
                        }
                    } else {
                        let a1890 = v0;
                        let a1891 = 6u64;
                        if nat_lt(a1890, a1891) {
                            let a1892 = v0;
                            let a1893 = 5u64;
                            if nat_lt(a1892, a1893) {
                                Adt15::C0(0u64, 4u64, Adt13::C0, 8u64, 12u64)
                            } else {
                                Adt15::C0(0u64, 5u64, Adt13::C2, 4u64, 8u64)
                            }
                        } else {
                            let a1894 = v0;
                            let a1895 = 7u64;
                            if nat_lt(a1894, a1895) {
                                Adt15::C0(0u64, 6u64, Adt13::C3, 4u64, 12u64)
                            } else {
                                Adt15::C0(0u64, 7u64, Adt13::C0, 0u64, 4u64)
                            }
                        }
                    }
                } else {
                    let a1896 = v0;
                    let a1897 = 12u64;
                    if nat_lt(a1896, a1897) {
                        let a1898 = v0;
                        let a1899 = 10u64;
                        if nat_lt(a1898, a1899) {
                            let a1900 = v0;
                            let a1901 = 9u64;
                            if nat_lt(a1900, a1901) {
                                Adt15::C0(0u64, 8u64, Adt13::C1, 0u64, 1u64)
                            } else {
                                Adt15::C0(0u64, 9u64, Adt13::C2, 12u64, 0u64)
                            }
                        } else {
                            let a1902 = v0;
                            let a1903 = 11u64;
                            if nat_lt(a1902, a1903) {
                                Adt15::C0(0u64, 10u64, Adt13::C3, 12u64, 8u64)
                            } else {
                                Adt15::C0(0u64, 11u64, Adt13::C0, 8u64, 12u64)
                            }
                        }
                    } else {
                        let a1904 = v0;
                        let a1905 = 14u64;
                        if nat_lt(a1904, a1905) {
                            let a1906 = v0;
                            let a1907 = 13u64;
                            if nat_lt(a1906, a1907) {
                                Adt15::C0(0u64, 12u64, Adt13::C2, 4u64, 8u64)
                            } else {
                                Adt15::C0(0u64, 13u64, Adt13::C3, 4u64, 7u64)
                            }
                        } else {
                            let a1908 = v0;
                            let a1909 = 15u64;
                            if nat_lt(a1908, a1909) {
                                Adt15::C0(1u64, 0u64, Adt13::C0, 1u64, 5u64)
                            } else {
                                Adt15::C0(1u64, 1u64, Adt13::C1, 1u64, 2u64)
                            }
                        }
                    }
                }
            } else {
                let a1910 = v0;
                let a1911 = 24u64;
                if nat_lt(a1910, a1911) {
                    let a1912 = v0;
                    let a1913 = 20u64;
                    if nat_lt(a1912, a1913) {
                        let a1914 = v0;
                        let a1915 = 18u64;
                        if nat_lt(a1914, a1915) {
                            let a1916 = v0;
                            let a1917 = 17u64;
                            if nat_lt(a1916, a1917) {
                                Adt15::C0(1u64, 2u64, Adt13::C2, 13u64, 1u64)
                            } else {
                                Adt15::C0(1u64, 3u64, Adt13::C3, 13u64, 16u64)
                            }
                        } else {
                            let a1918 = v0;
                            let a1919 = 19u64;
                            if nat_lt(a1918, a1919) {
                                Adt15::C0(1u64, 4u64, Adt13::C0, 9u64, 13u64)
                            } else {
                                Adt15::C0(1u64, 5u64, Adt13::C2, 5u64, 9u64)
                            }
                        }
                    } else {
                        let a1920 = v0;
                        let a1921 = 22u64;
                        if nat_lt(a1920, a1921) {
                            let a1922 = v0;
                            let a1923 = 21u64;
                            if nat_lt(a1922, a1923) {
                                Adt15::C0(1u64, 6u64, Adt13::C3, 5u64, 12u64)
                            } else {
                                Adt15::C0(1u64, 7u64, Adt13::C0, 1u64, 5u64)
                            }
                        } else {
                            let a1924 = v0;
                            let a1925 = 23u64;
                            if nat_lt(a1924, a1925) {
                                Adt15::C0(1u64, 8u64, Adt13::C1, 1u64, 3u64)
                            } else {
                                Adt15::C0(1u64, 9u64, Adt13::C2, 13u64, 1u64)
                            }
                        }
                    }
                } else {
                    let a1926 = v0;
                    let a1927 = 28u64;
                    if nat_lt(a1926, a1927) {
                        let a1928 = v0;
                        let a1929 = 26u64;
                        if nat_lt(a1928, a1929) {
                            let a1930 = v0;
                            let a1931 = 25u64;
                            if nat_lt(a1930, a1931) {
                                Adt15::C0(1u64, 10u64, Adt13::C3, 13u64, 8u64)
                            } else {
                                Adt15::C0(1u64, 11u64, Adt13::C0, 9u64, 13u64)
                            }
                        } else {
                            let a1932 = v0;
                            let a1933 = 27u64;
                            if nat_lt(a1932, a1933) {
                                Adt15::C0(1u64, 12u64, Adt13::C2, 5u64, 9u64)
                            } else {
                                Adt15::C0(1u64, 13u64, Adt13::C3, 5u64, 7u64)
                            }
                        }
                    } else {
                        let a1934 = v0;
                        let a1935 = 30u64;
                        if nat_lt(a1934, a1935) {
                            let a1936 = v0;
                            let a1937 = 29u64;
                            if nat_lt(a1936, a1937) {
                                Adt15::C0(2u64, 0u64, Adt13::C0, 2u64, 6u64)
                            } else {
                                Adt15::C0(2u64, 1u64, Adt13::C1, 2u64, 4u64)
                            }
                        } else {
                            let a1938 = v0;
                            let a1939 = 31u64;
                            if nat_lt(a1938, a1939) {
                                Adt15::C0(2u64, 2u64, Adt13::C2, 14u64, 2u64)
                            } else {
                                Adt15::C0(2u64, 3u64, Adt13::C3, 14u64, 16u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a1940 = v0;
            let a1941 = 48u64;
            if nat_lt(a1940, a1941) {
                let a1942 = v0;
                let a1943 = 40u64;
                if nat_lt(a1942, a1943) {
                    let a1944 = v0;
                    let a1945 = 36u64;
                    if nat_lt(a1944, a1945) {
                        let a1946 = v0;
                        let a1947 = 34u64;
                        if nat_lt(a1946, a1947) {
                            let a1948 = v0;
                            let a1949 = 33u64;
                            if nat_lt(a1948, a1949) {
                                Adt15::C0(2u64, 4u64, Adt13::C0, 10u64, 14u64)
                            } else {
                                Adt15::C0(2u64, 5u64, Adt13::C2, 6u64, 10u64)
                            }
                        } else {
                            let a1950 = v0;
                            let a1951 = 35u64;
                            if nat_lt(a1950, a1951) {
                                Adt15::C0(2u64, 6u64, Adt13::C3, 6u64, 12u64)
                            } else {
                                Adt15::C0(2u64, 7u64, Adt13::C0, 2u64, 6u64)
                            }
                        }
                    } else {
                        let a1952 = v0;
                        let a1953 = 38u64;
                        if nat_lt(a1952, a1953) {
                            let a1954 = v0;
                            let a1955 = 37u64;
                            if nat_lt(a1954, a1955) {
                                Adt15::C0(2u64, 8u64, Adt13::C1, 2u64, 5u64)
                            } else {
                                Adt15::C0(2u64, 9u64, Adt13::C2, 14u64, 2u64)
                            }
                        } else {
                            let a1956 = v0;
                            let a1957 = 39u64;
                            if nat_lt(a1956, a1957) {
                                Adt15::C0(2u64, 10u64, Adt13::C3, 14u64, 8u64)
                            } else {
                                Adt15::C0(2u64, 11u64, Adt13::C0, 10u64, 14u64)
                            }
                        }
                    }
                } else {
                    let a1958 = v0;
                    let a1959 = 44u64;
                    if nat_lt(a1958, a1959) {
                        let a1960 = v0;
                        let a1961 = 42u64;
                        if nat_lt(a1960, a1961) {
                            let a1962 = v0;
                            let a1963 = 41u64;
                            if nat_lt(a1962, a1963) {
                                Adt15::C0(2u64, 12u64, Adt13::C2, 6u64, 10u64)
                            } else {
                                Adt15::C0(2u64, 13u64, Adt13::C3, 6u64, 7u64)
                            }
                        } else {
                            let a1964 = v0;
                            let a1965 = 43u64;
                            if nat_lt(a1964, a1965) {
                                Adt15::C0(3u64, 0u64, Adt13::C0, 3u64, 7u64)
                            } else {
                                Adt15::C0(3u64, 1u64, Adt13::C1, 3u64, 6u64)
                            }
                        }
                    } else {
                        let a1966 = v0;
                        let a1967 = 46u64;
                        if nat_lt(a1966, a1967) {
                            let a1968 = v0;
                            let a1969 = 45u64;
                            if nat_lt(a1968, a1969) {
                                Adt15::C0(3u64, 2u64, Adt13::C2, 15u64, 3u64)
                            } else {
                                Adt15::C0(3u64, 3u64, Adt13::C3, 15u64, 16u64)
                            }
                        } else {
                            let a1970 = v0;
                            let a1971 = 47u64;
                            if nat_lt(a1970, a1971) {
                                Adt15::C0(3u64, 4u64, Adt13::C0, 11u64, 15u64)
                            } else {
                                Adt15::C0(3u64, 5u64, Adt13::C2, 7u64, 11u64)
                            }
                        }
                    }
                }
            } else {
                let a1972 = v0;
                let a1973 = 56u64;
                if nat_lt(a1972, a1973) {
                    let a1974 = v0;
                    let a1975 = 52u64;
                    if nat_lt(a1974, a1975) {
                        let a1976 = v0;
                        let a1977 = 50u64;
                        if nat_lt(a1976, a1977) {
                            let a1978 = v0;
                            let a1979 = 49u64;
                            if nat_lt(a1978, a1979) {
                                Adt15::C0(3u64, 6u64, Adt13::C3, 7u64, 12u64)
                            } else {
                                Adt15::C0(3u64, 7u64, Adt13::C0, 3u64, 7u64)
                            }
                        } else {
                            let a1980 = v0;
                            let a1981 = 51u64;
                            if nat_lt(a1980, a1981) {
                                Adt15::C0(3u64, 8u64, Adt13::C1, 3u64, 7u64)
                            } else {
                                Adt15::C0(3u64, 9u64, Adt13::C2, 15u64, 3u64)
                            }
                        }
                    } else {
                        let a1982 = v0;
                        let a1983 = 54u64;
                        if nat_lt(a1982, a1983) {
                            let a1984 = v0;
                            let a1985 = 53u64;
                            if nat_lt(a1984, a1985) {
                                Adt15::C0(3u64, 10u64, Adt13::C3, 15u64, 8u64)
                            } else {
                                Adt15::C0(3u64, 11u64, Adt13::C0, 11u64, 15u64)
                            }
                        } else {
                            let a1986 = v0;
                            let a1987 = 55u64;
                            if nat_lt(a1986, a1987) {
                                Adt15::C0(3u64, 12u64, Adt13::C2, 7u64, 11u64)
                            } else {
                                Adt15::C0(3u64, 13u64, Adt13::C3, 7u64, 7u64)
                            }
                        }
                    }
                } else {
                    let a1988 = v0;
                    let a1989 = 60u64;
                    if nat_lt(a1988, a1989) {
                        let a1990 = v0;
                        let a1991 = 58u64;
                        if nat_lt(a1990, a1991) {
                            let a1992 = v0;
                            let a1993 = 57u64;
                            if nat_lt(a1992, a1993) {
                                Adt15::C0(4u64, 0u64, Adt13::C0, 0u64, 5u64)
                            } else {
                                Adt15::C0(4u64, 1u64, Adt13::C1, 0u64, 8u64)
                            }
                        } else {
                            let a1994 = v0;
                            let a1995 = 59u64;
                            if nat_lt(a1994, a1995) {
                                Adt15::C0(4u64, 2u64, Adt13::C2, 15u64, 0u64)
                            } else {
                                Adt15::C0(4u64, 3u64, Adt13::C3, 15u64, 16u64)
                            }
                        }
                    } else {
                        let a1996 = v0;
                        let a1997 = 62u64;
                        if nat_lt(a1996, a1997) {
                            let a1998 = v0;
                            let a1999 = 61u64;
                            if nat_lt(a1998, a1999) {
                                Adt15::C0(4u64, 4u64, Adt13::C0, 10u64, 15u64)
                            } else {
                                Adt15::C0(4u64, 5u64, Adt13::C2, 5u64, 10u64)
                            }
                        } else {
                            let a2000 = v0;
                            let a2001 = 63u64;
                            if nat_lt(a2000, a2001) {
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
        let a2002 = v0;
        let a2003 = 96u64;
        if nat_lt(a2002, a2003) {
            let a2004 = v0;
            let a2005 = 80u64;
            if nat_lt(a2004, a2005) {
                let a2006 = v0;
                let a2007 = 72u64;
                if nat_lt(a2006, a2007) {
                    let a2008 = v0;
                    let a2009 = 68u64;
                    if nat_lt(a2008, a2009) {
                        let a2010 = v0;
                        let a2011 = 66u64;
                        if nat_lt(a2010, a2011) {
                            let a2012 = v0;
                            let a2013 = 65u64;
                            if nat_lt(a2012, a2013) {
                                Adt15::C0(4u64, 8u64, Adt13::C1, 0u64, 9u64)
                            } else {
                                Adt15::C0(4u64, 9u64, Adt13::C2, 15u64, 0u64)
                            }
                        } else {
                            let a2014 = v0;
                            let a2015 = 67u64;
                            if nat_lt(a2014, a2015) {
                                Adt15::C0(4u64, 10u64, Adt13::C3, 15u64, 8u64)
                            } else {
                                Adt15::C0(4u64, 11u64, Adt13::C0, 10u64, 15u64)
                            }
                        }
                    } else {
                        let a2016 = v0;
                        let a2017 = 70u64;
                        if nat_lt(a2016, a2017) {
                            let a2018 = v0;
                            let a2019 = 69u64;
                            if nat_lt(a2018, a2019) {
                                Adt15::C0(4u64, 12u64, Adt13::C2, 5u64, 10u64)
                            } else {
                                Adt15::C0(4u64, 13u64, Adt13::C3, 5u64, 7u64)
                            }
                        } else {
                            let a2020 = v0;
                            let a2021 = 71u64;
                            if nat_lt(a2020, a2021) {
                                Adt15::C0(5u64, 0u64, Adt13::C0, 1u64, 6u64)
                            } else {
                                Adt15::C0(5u64, 1u64, Adt13::C1, 1u64, 10u64)
                            }
                        }
                    }
                } else {
                    let a2022 = v0;
                    let a2023 = 76u64;
                    if nat_lt(a2022, a2023) {
                        let a2024 = v0;
                        let a2025 = 74u64;
                        if nat_lt(a2024, a2025) {
                            let a2026 = v0;
                            let a2027 = 73u64;
                            if nat_lt(a2026, a2027) {
                                Adt15::C0(5u64, 2u64, Adt13::C2, 12u64, 1u64)
                            } else {
                                Adt15::C0(5u64, 3u64, Adt13::C3, 12u64, 16u64)
                            }
                        } else {
                            let a2028 = v0;
                            let a2029 = 75u64;
                            if nat_lt(a2028, a2029) {
                                Adt15::C0(5u64, 4u64, Adt13::C0, 11u64, 12u64)
                            } else {
                                Adt15::C0(5u64, 5u64, Adt13::C2, 6u64, 11u64)
                            }
                        }
                    } else {
                        let a2030 = v0;
                        let a2031 = 78u64;
                        if nat_lt(a2030, a2031) {
                            let a2032 = v0;
                            let a2033 = 77u64;
                            if nat_lt(a2032, a2033) {
                                Adt15::C0(5u64, 6u64, Adt13::C3, 6u64, 12u64)
                            } else {
                                Adt15::C0(5u64, 7u64, Adt13::C0, 1u64, 6u64)
                            }
                        } else {
                            let a2034 = v0;
                            let a2035 = 79u64;
                            if nat_lt(a2034, a2035) {
                                Adt15::C0(5u64, 8u64, Adt13::C1, 1u64, 11u64)
                            } else {
                                Adt15::C0(5u64, 9u64, Adt13::C2, 12u64, 1u64)
                            }
                        }
                    }
                }
            } else {
                let a2036 = v0;
                let a2037 = 88u64;
                if nat_lt(a2036, a2037) {
                    let a2038 = v0;
                    let a2039 = 84u64;
                    if nat_lt(a2038, a2039) {
                        let a2040 = v0;
                        let a2041 = 82u64;
                        if nat_lt(a2040, a2041) {
                            let a2042 = v0;
                            let a2043 = 81u64;
                            if nat_lt(a2042, a2043) {
                                Adt15::C0(5u64, 10u64, Adt13::C3, 12u64, 8u64)
                            } else {
                                Adt15::C0(5u64, 11u64, Adt13::C0, 11u64, 12u64)
                            }
                        } else {
                            let a2044 = v0;
                            let a2045 = 83u64;
                            if nat_lt(a2044, a2045) {
                                Adt15::C0(5u64, 12u64, Adt13::C2, 6u64, 11u64)
                            } else {
                                Adt15::C0(5u64, 13u64, Adt13::C3, 6u64, 7u64)
                            }
                        }
                    } else {
                        let a2046 = v0;
                        let a2047 = 86u64;
                        if nat_lt(a2046, a2047) {
                            let a2048 = v0;
                            let a2049 = 85u64;
                            if nat_lt(a2048, a2049) {
                                Adt15::C0(6u64, 0u64, Adt13::C0, 2u64, 7u64)
                            } else {
                                Adt15::C0(6u64, 1u64, Adt13::C1, 2u64, 12u64)
                            }
                        } else {
                            let a2050 = v0;
                            let a2051 = 87u64;
                            if nat_lt(a2050, a2051) {
                                Adt15::C0(6u64, 2u64, Adt13::C2, 13u64, 2u64)
                            } else {
                                Adt15::C0(6u64, 3u64, Adt13::C3, 13u64, 16u64)
                            }
                        }
                    }
                } else {
                    let a2052 = v0;
                    let a2053 = 92u64;
                    if nat_lt(a2052, a2053) {
                        let a2054 = v0;
                        let a2055 = 90u64;
                        if nat_lt(a2054, a2055) {
                            let a2056 = v0;
                            let a2057 = 89u64;
                            if nat_lt(a2056, a2057) {
                                Adt15::C0(6u64, 4u64, Adt13::C0, 8u64, 13u64)
                            } else {
                                Adt15::C0(6u64, 5u64, Adt13::C2, 7u64, 8u64)
                            }
                        } else {
                            let a2058 = v0;
                            let a2059 = 91u64;
                            if nat_lt(a2058, a2059) {
                                Adt15::C0(6u64, 6u64, Adt13::C3, 7u64, 12u64)
                            } else {
                                Adt15::C0(6u64, 7u64, Adt13::C0, 2u64, 7u64)
                            }
                        }
                    } else {
                        let a2060 = v0;
                        let a2061 = 94u64;
                        if nat_lt(a2060, a2061) {
                            let a2062 = v0;
                            let a2063 = 93u64;
                            if nat_lt(a2062, a2063) {
                                Adt15::C0(6u64, 8u64, Adt13::C1, 2u64, 13u64)
                            } else {
                                Adt15::C0(6u64, 9u64, Adt13::C2, 13u64, 2u64)
                            }
                        } else {
                            let a2064 = v0;
                            let a2065 = 95u64;
                            if nat_lt(a2064, a2065) {
                                Adt15::C0(6u64, 10u64, Adt13::C3, 13u64, 8u64)
                            } else {
                                Adt15::C0(6u64, 11u64, Adt13::C0, 8u64, 13u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a2066 = v0;
            let a2067 = 112u64;
            if nat_lt(a2066, a2067) {
                let a2068 = v0;
                let a2069 = 104u64;
                if nat_lt(a2068, a2069) {
                    let a2070 = v0;
                    let a2071 = 100u64;
                    if nat_lt(a2070, a2071) {
                        let a2072 = v0;
                        let a2073 = 98u64;
                        if nat_lt(a2072, a2073) {
                            let a2074 = v0;
                            let a2075 = 97u64;
                            if nat_lt(a2074, a2075) {
                                Adt15::C0(6u64, 12u64, Adt13::C2, 7u64, 8u64)
                            } else {
                                Adt15::C0(6u64, 13u64, Adt13::C3, 7u64, 7u64)
                            }
                        } else {
                            let a2076 = v0;
                            let a2077 = 99u64;
                            if nat_lt(a2076, a2077) {
                                Adt15::C0(7u64, 0u64, Adt13::C0, 3u64, 4u64)
                            } else {
                                Adt15::C0(7u64, 1u64, Adt13::C1, 3u64, 14u64)
                            }
                        }
                    } else {
                        let a2078 = v0;
                        let a2079 = 102u64;
                        if nat_lt(a2078, a2079) {
                            let a2080 = v0;
                            let a2081 = 101u64;
                            if nat_lt(a2080, a2081) {
                                Adt15::C0(7u64, 2u64, Adt13::C2, 14u64, 3u64)
                            } else {
                                Adt15::C0(7u64, 3u64, Adt13::C3, 14u64, 16u64)
                            }
                        } else {
                            let a2082 = v0;
                            let a2083 = 103u64;
                            if nat_lt(a2082, a2083) {
                                Adt15::C0(7u64, 4u64, Adt13::C0, 9u64, 14u64)
                            } else {
                                Adt15::C0(7u64, 5u64, Adt13::C2, 4u64, 9u64)
                            }
                        }
                    }
                } else {
                    let a2084 = v0;
                    let a2085 = 108u64;
                    if nat_lt(a2084, a2085) {
                        let a2086 = v0;
                        let a2087 = 106u64;
                        if nat_lt(a2086, a2087) {
                            let a2088 = v0;
                            let a2089 = 105u64;
                            if nat_lt(a2088, a2089) {
                                Adt15::C0(7u64, 6u64, Adt13::C3, 4u64, 12u64)
                            } else {
                                Adt15::C0(7u64, 7u64, Adt13::C0, 3u64, 4u64)
                            }
                        } else {
                            let a2090 = v0;
                            let a2091 = 107u64;
                            if nat_lt(a2090, a2091) {
                                Adt15::C0(7u64, 8u64, Adt13::C1, 3u64, 15u64)
                            } else {
                                Adt15::C0(7u64, 9u64, Adt13::C2, 14u64, 3u64)
                            }
                        }
                    } else {
                        let a2092 = v0;
                        let a2093 = 110u64;
                        if nat_lt(a2092, a2093) {
                            let a2094 = v0;
                            let a2095 = 109u64;
                            if nat_lt(a2094, a2095) {
                                Adt15::C0(7u64, 10u64, Adt13::C3, 14u64, 8u64)
                            } else {
                                Adt15::C0(7u64, 11u64, Adt13::C0, 9u64, 14u64)
                            }
                        } else {
                            let a2096 = v0;
                            let a2097 = 111u64;
                            if nat_lt(a2096, a2097) {
                                Adt15::C0(7u64, 12u64, Adt13::C2, 4u64, 9u64)
                            } else {
                                Adt15::C0(7u64, 13u64, Adt13::C3, 4u64, 7u64)
                            }
                        }
                    }
                }
            } else {
                let a2098 = v0;
                let a2099 = 120u64;
                if nat_lt(a2098, a2099) {
                    let a2100 = v0;
                    let a2101 = 116u64;
                    if nat_lt(a2100, a2101) {
                        let a2102 = v0;
                        let a2103 = 114u64;
                        if nat_lt(a2102, a2103) {
                            let a2104 = v0;
                            let a2105 = 113u64;
                            if nat_lt(a2104, a2105) {
                                Adt15::C0(0u64, 0u64, Adt13::C2, 0u64, 8u64)
                            } else {
                                Adt15::C0(0u64, 1u64, Adt13::C2, 1u64, 9u64)
                            }
                        } else {
                            let a2106 = v0;
                            let a2107 = 115u64;
                            if nat_lt(a2106, a2107) {
                                Adt15::C0(0u64, 2u64, Adt13::C2, 2u64, 10u64)
                            } else {
                                Adt15::C0(0u64, 3u64, Adt13::C2, 3u64, 11u64)
                            }
                        }
                    } else {
                        let a2108 = v0;
                        let a2109 = 118u64;
                        if nat_lt(a2108, a2109) {
                            let a2110 = v0;
                            let a2111 = 117u64;
                            if nat_lt(a2110, a2111) {
                                Adt15::C0(0u64, 4u64, Adt13::C2, 4u64, 12u64)
                            } else {
                                Adt15::C0(0u64, 5u64, Adt13::C2, 5u64, 13u64)
                            }
                        } else {
                            let a2112 = v0;
                            let a2113 = 119u64;
                            if nat_lt(a2112, a2113) {
                                Adt15::C0(0u64, 6u64, Adt13::C2, 6u64, 14u64)
                            } else {
                                Adt15::C0(0u64, 7u64, Adt13::C2, 7u64, 15u64)
                            }
                        }
                    }
                } else {
                    let a2114 = v0;
                    let a2115 = 124u64;
                    if nat_lt(a2114, a2115) {
                        let a2116 = v0;
                        let a2117 = 122u64;
                        if nat_lt(a2116, a2117) {
                            let a2118 = v0;
                            let a2119 = 121u64;
                            if nat_lt(a2118, a2119) {
                                Adt15::C0(0u64, 8u64, Adt13::C4, 8u64, 0u64)
                            } else {
                                Adt15::C0(0u64, 9u64, Adt13::C4, 9u64, 1u64)
                            }
                        } else {
                            let a2120 = v0;
                            let a2121 = 123u64;
                            if nat_lt(a2120, a2121) {
                                Adt15::C0(0u64, 10u64, Adt13::C4, 10u64, 2u64)
                            } else {
                                Adt15::C0(0u64, 11u64, Adt13::C4, 11u64, 3u64)
                            }
                        }
                    } else {
                        let a2122 = v0;
                        let a2123 = 126u64;
                        if nat_lt(a2122, a2123) {
                            let a2124 = v0;
                            let a2125 = 125u64;
                            if nat_lt(a2124, a2125) {
                                Adt15::C0(0u64, 12u64, Adt13::C4, 12u64, 4u64)
                            } else {
                                Adt15::C0(0u64, 13u64, Adt13::C4, 13u64, 5u64)
                            }
                        } else {
                            let a2126 = v0;
                            let a2127 = 127u64;
                            if nat_lt(a2126, a2127) {
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

pub fn f83(v0: Adt6, v1: u64) -> u32 {
    let a2128 = v1;
    let a2129 = 8u64;
    if nat_lt(a2128, a2129) {
        let a2130 = v1;
        let a2131 = 4u64;
        if nat_lt(a2130, a2131) {
            let a2132 = v1;
            let a2133 = 2u64;
            if nat_lt(a2132, a2133) {
                let a2134 = v1;
                let a2135 = 1u64;
                if nat_lt(a2134, a2135) {
                    match v0 {
                        Adt6::C0(h2136, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2136
                        }
                    }
                } else {
                    match v0 {
                        Adt6::C0(_, h2137, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2137
                        }
                    }
                }
            } else {
                let a2138 = v1;
                let a2139 = 3u64;
                if nat_lt(a2138, a2139) {
                    match v0 {
                        Adt6::C0(_, _, h2140, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2140
                        }
                    }
                } else {
                    match v0 {
                        Adt6::C0(_, _, _, h2141, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2141
                        }
                    }
                }
            }
        } else {
            let a2142 = v1;
            let a2143 = 6u64;
            if nat_lt(a2142, a2143) {
                let a2144 = v1;
                let a2145 = 5u64;
                if nat_lt(a2144, a2145) {
                    match v0 {
                        Adt6::C0(_, _, _, _, h2146, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2146
                        }
                    }
                } else {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, h2147, _, _, _, _, _, _, _, _, _, _) => {
                            h2147
                        }
                    }
                }
            } else {
                let a2148 = v1;
                let a2149 = 7u64;
                if nat_lt(a2148, a2149) {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, h2150, _, _, _, _, _, _, _, _, _) => {
                            h2150
                        }
                    }
                } else {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, h2151, _, _, _, _, _, _, _, _) => {
                            h2151
                        }
                    }
                }
            }
        }
    } else {
        let a2152 = v1;
        let a2153 = 12u64;
        if nat_lt(a2152, a2153) {
            let a2154 = v1;
            let a2155 = 10u64;
            if nat_lt(a2154, a2155) {
                let a2156 = v1;
                let a2157 = 9u64;
                if nat_lt(a2156, a2157) {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2158, _, _, _, _, _, _, _) => {
                            h2158
                        }
                    }
                } else {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2159, _, _, _, _, _, _) => {
                            h2159
                        }
                    }
                }
            } else {
                let a2160 = v1;
                let a2161 = 11u64;
                if nat_lt(a2160, a2161) {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2162, _, _, _, _, _) => {
                            h2162
                        }
                    }
                } else {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2163, _, _, _, _) => {
                            h2163
                        }
                    }
                }
            }
        } else {
            let a2164 = v1;
            let a2165 = 14u64;
            if nat_lt(a2164, a2165) {
                let a2166 = v1;
                let a2167 = 13u64;
                if nat_lt(a2166, a2167) {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2168, _, _, _) => {
                            h2168
                        }
                    }
                } else {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2169, _, _) => {
                            h2169
                        }
                    }
                }
            } else {
                let a2170 = v1;
                let a2171 = 15u64;
                if nat_lt(a2170, a2171) {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2172, _) => {
                            h2172
                        }
                    }
                } else {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2173) => {
                            h2173
                        }
                    }
                }
            }
        }
    }
}

pub fn f84(v0: u64, v1: u64) -> R<u64> {
    let v2: u64 = {
        let a2176 = {
            let a2174 = v0;
            let a2175 = 16u64;
            nat_mul(a2174, a2175)?
        };
        let a2177 = v1;
        nat_add(a2176, a2177)?
    };
    let a2178 = v2;
    let a2179 = 56u64;
    if nat_lt(a2178, a2179) {
        let a2180 = v2;
        let a2181 = 28u64;
        if nat_lt(a2180, a2181) {
            let a2182 = v2;
            let a2183 = 14u64;
            if nat_lt(a2182, a2183) {
                let a2184 = v2;
                let a2185 = 7u64;
                if nat_lt(a2184, a2185) {
                    let a2186 = v2;
                    let a2187 = 3u64;
                    if nat_lt(a2186, a2187) {
                        let a2188 = v2;
                        let a2189 = 1u64;
                        if nat_lt(a2188, a2189) {
                            Ok(0u64)
                        } else {
                            let a2190 = v2;
                            let a2191 = 2u64;
                            if nat_lt(a2190, a2191) {
                                Ok(1u64)
                            } else {
                                Ok(2u64)
                            }
                        }
                    } else {
                        let a2192 = v2;
                        let a2193 = 5u64;
                        if nat_lt(a2192, a2193) {
                            let a2194 = v2;
                            let a2195 = 4u64;
                            if nat_lt(a2194, a2195) {
                                Ok(3u64)
                            } else {
                                Ok(4u64)
                            }
                        } else {
                            let a2196 = v2;
                            let a2197 = 6u64;
                            if nat_lt(a2196, a2197) {
                                Ok(5u64)
                            } else {
                                Ok(6u64)
                            }
                        }
                    }
                } else {
                    let a2198 = v2;
                    let a2199 = 10u64;
                    if nat_lt(a2198, a2199) {
                        let a2200 = v2;
                        let a2201 = 8u64;
                        if nat_lt(a2200, a2201) {
                            Ok(7u64)
                        } else {
                            let a2202 = v2;
                            let a2203 = 9u64;
                            if nat_lt(a2202, a2203) {
                                Ok(8u64)
                            } else {
                                Ok(9u64)
                            }
                        }
                    } else {
                        let a2204 = v2;
                        let a2205 = 12u64;
                        if nat_lt(a2204, a2205) {
                            let a2206 = v2;
                            let a2207 = 11u64;
                            if nat_lt(a2206, a2207) {
                                Ok(10u64)
                            } else {
                                Ok(11u64)
                            }
                        } else {
                            let a2208 = v2;
                            let a2209 = 13u64;
                            if nat_lt(a2208, a2209) {
                                Ok(12u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    }
                }
            } else {
                let a2210 = v2;
                let a2211 = 21u64;
                if nat_lt(a2210, a2211) {
                    let a2212 = v2;
                    let a2213 = 17u64;
                    if nat_lt(a2212, a2213) {
                        let a2214 = v2;
                        let a2215 = 15u64;
                        if nat_lt(a2214, a2215) {
                            Ok(14u64)
                        } else {
                            let a2216 = v2;
                            let a2217 = 16u64;
                            if nat_lt(a2216, a2217) {
                                Ok(15u64)
                            } else {
                                Ok(2u64)
                            }
                        }
                    } else {
                        let a2218 = v2;
                        let a2219 = 19u64;
                        if nat_lt(a2218, a2219) {
                            let a2220 = v2;
                            let a2221 = 18u64;
                            if nat_lt(a2220, a2221) {
                                Ok(6u64)
                            } else {
                                Ok(3u64)
                            }
                        } else {
                            let a2222 = v2;
                            let a2223 = 20u64;
                            if nat_lt(a2222, a2223) {
                                Ok(10u64)
                            } else {
                                Ok(7u64)
                            }
                        }
                    }
                } else {
                    let a2224 = v2;
                    let a2225 = 24u64;
                    if nat_lt(a2224, a2225) {
                        let a2226 = v2;
                        let a2227 = 22u64;
                        if nat_lt(a2226, a2227) {
                            Ok(0u64)
                        } else {
                            let a2228 = v2;
                            let a2229 = 23u64;
                            if nat_lt(a2228, a2229) {
                                Ok(4u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    } else {
                        let a2230 = v2;
                        let a2231 = 26u64;
                        if nat_lt(a2230, a2231) {
                            let a2232 = v2;
                            let a2233 = 25u64;
                            if nat_lt(a2232, a2233) {
                                Ok(1u64)
                            } else {
                                Ok(11u64)
                            }
                        } else {
                            let a2234 = v2;
                            let a2235 = 27u64;
                            if nat_lt(a2234, a2235) {
                                Ok(12u64)
                            } else {
                                Ok(5u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a2236 = v2;
            let a2237 = 42u64;
            if nat_lt(a2236, a2237) {
                let a2238 = v2;
                let a2239 = 35u64;
                if nat_lt(a2238, a2239) {
                    let a2240 = v2;
                    let a2241 = 31u64;
                    if nat_lt(a2240, a2241) {
                        let a2242 = v2;
                        let a2243 = 29u64;
                        if nat_lt(a2242, a2243) {
                            Ok(9u64)
                        } else {
                            let a2244 = v2;
                            let a2245 = 30u64;
                            if nat_lt(a2244, a2245) {
                                Ok(14u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    } else {
                        let a2246 = v2;
                        let a2247 = 33u64;
                        if nat_lt(a2246, a2247) {
                            let a2248 = v2;
                            let a2249 = 32u64;
                            if nat_lt(a2248, a2249) {
                                Ok(8u64)
                            } else {
                                Ok(3u64)
                            }
                        } else {
                            let a2250 = v2;
                            let a2251 = 34u64;
                            if nat_lt(a2250, a2251) {
                                Ok(4u64)
                            } else {
                                Ok(10u64)
                            }
                        }
                    }
                } else {
                    let a2252 = v2;
                    let a2253 = 38u64;
                    if nat_lt(a2252, a2253) {
                        let a2254 = v2;
                        let a2255 = 36u64;
                        if nat_lt(a2254, a2255) {
                            Ok(12u64)
                        } else {
                            let a2256 = v2;
                            let a2257 = 37u64;
                            if nat_lt(a2256, a2257) {
                                Ok(13u64)
                            } else {
                                Ok(2u64)
                            }
                        }
                    } else {
                        let a2258 = v2;
                        let a2259 = 40u64;
                        if nat_lt(a2258, a2259) {
                            let a2260 = v2;
                            let a2261 = 39u64;
                            if nat_lt(a2260, a2261) {
                                Ok(7u64)
                            } else {
                                Ok(14u64)
                            }
                        } else {
                            let a2262 = v2;
                            let a2263 = 41u64;
                            if nat_lt(a2262, a2263) {
                                Ok(6u64)
                            } else {
                                Ok(5u64)
                            }
                        }
                    }
                }
            } else {
                let a2264 = v2;
                let a2265 = 49u64;
                if nat_lt(a2264, a2265) {
                    let a2266 = v2;
                    let a2267 = 45u64;
                    if nat_lt(a2266, a2267) {
                        let a2268 = v2;
                        let a2269 = 43u64;
                        if nat_lt(a2268, a2269) {
                            Ok(9u64)
                        } else {
                            let a2270 = v2;
                            let a2271 = 44u64;
                            if nat_lt(a2270, a2271) {
                                Ok(0u64)
                            } else {
                                Ok(11u64)
                            }
                        }
                    } else {
                        let a2272 = v2;
                        let a2273 = 47u64;
                        if nat_lt(a2272, a2273) {
                            let a2274 = v2;
                            let a2275 = 46u64;
                            if nat_lt(a2274, a2275) {
                                Ok(15u64)
                            } else {
                                Ok(8u64)
                            }
                        } else {
                            let a2276 = v2;
                            let a2277 = 48u64;
                            if nat_lt(a2276, a2277) {
                                Ok(1u64)
                            } else {
                                Ok(10u64)
                            }
                        }
                    }
                } else {
                    let a2278 = v2;
                    let a2279 = 52u64;
                    if nat_lt(a2278, a2279) {
                        let a2280 = v2;
                        let a2281 = 50u64;
                        if nat_lt(a2280, a2281) {
                            Ok(7u64)
                        } else {
                            let a2282 = v2;
                            let a2283 = 51u64;
                            if nat_lt(a2282, a2283) {
                                Ok(12u64)
                            } else {
                                Ok(9u64)
                            }
                        }
                    } else {
                        let a2284 = v2;
                        let a2285 = 54u64;
                        if nat_lt(a2284, a2285) {
                            let a2286 = v2;
                            let a2287 = 53u64;
                            if nat_lt(a2286, a2287) {
                                Ok(14u64)
                            } else {
                                Ok(3u64)
                            }
                        } else {
                            let a2288 = v2;
                            let a2289 = 55u64;
                            if nat_lt(a2288, a2289) {
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
        let a2290 = v2;
        let a2291 = 84u64;
        if nat_lt(a2290, a2291) {
            let a2292 = v2;
            let a2293 = 70u64;
            if nat_lt(a2292, a2293) {
                let a2294 = v2;
                let a2295 = 63u64;
                if nat_lt(a2294, a2295) {
                    let a2296 = v2;
                    let a2297 = 59u64;
                    if nat_lt(a2296, a2297) {
                        let a2298 = v2;
                        let a2299 = 57u64;
                        if nat_lt(a2298, a2299) {
                            Ok(4u64)
                        } else {
                            let a2300 = v2;
                            let a2301 = 58u64;
                            if nat_lt(a2300, a2301) {
                                Ok(0u64)
                            } else {
                                Ok(11u64)
                            }
                        }
                    } else {
                        let a2302 = v2;
                        let a2303 = 61u64;
                        if nat_lt(a2302, a2303) {
                            let a2304 = v2;
                            let a2305 = 60u64;
                            if nat_lt(a2304, a2305) {
                                Ok(2u64)
                            } else {
                                Ok(5u64)
                            }
                        } else {
                            let a2306 = v2;
                            let a2307 = 62u64;
                            if nat_lt(a2306, a2307) {
                                Ok(8u64)
                            } else {
                                Ok(1u64)
                            }
                        }
                    }
                } else {
                    let a2308 = v2;
                    let a2309 = 66u64;
                    if nat_lt(a2308, a2309) {
                        let a2310 = v2;
                        let a2311 = 64u64;
                        if nat_lt(a2310, a2311) {
                            Ok(6u64)
                        } else {
                            let a2312 = v2;
                            let a2313 = 65u64;
                            if nat_lt(a2312, a2313) {
                                Ok(12u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    } else {
                        let a2314 = v2;
                        let a2315 = 68u64;
                        if nat_lt(a2314, a2315) {
                            let a2316 = v2;
                            let a2317 = 67u64;
                            if nat_lt(a2316, a2317) {
                                Ok(9u64)
                            } else {
                                Ok(11u64)
                            }
                        } else {
                            let a2318 = v2;
                            let a2319 = 69u64;
                            if nat_lt(a2318, a2319) {
                                Ok(15u64)
                            } else {
                                Ok(10u64)
                            }
                        }
                    }
                }
            } else {
                let a2320 = v2;
                let a2321 = 77u64;
                if nat_lt(a2320, a2321) {
                    let a2322 = v2;
                    let a2323 = 73u64;
                    if nat_lt(a2322, a2323) {
                        let a2324 = v2;
                        let a2325 = 71u64;
                        if nat_lt(a2324, a2325) {
                            Ok(14u64)
                        } else {
                            let a2326 = v2;
                            let a2327 = 72u64;
                            if nat_lt(a2326, a2327) {
                                Ok(8u64)
                            } else {
                                Ok(7u64)
                            }
                        }
                    } else {
                        let a2328 = v2;
                        let a2329 = 75u64;
                        if nat_lt(a2328, a2329) {
                            let a2330 = v2;
                            let a2331 = 74u64;
                            if nat_lt(a2330, a2331) {
                                Ok(2u64)
                            } else {
                                Ok(5u64)
                            }
                        } else {
                            let a2332 = v2;
                            let a2333 = 76u64;
                            if nat_lt(a2332, a2333) {
                                Ok(3u64)
                            } else {
                                Ok(0u64)
                            }
                        }
                    }
                } else {
                    let a2334 = v2;
                    let a2335 = 80u64;
                    if nat_lt(a2334, a2335) {
                        let a2336 = v2;
                        let a2337 = 78u64;
                        if nat_lt(a2336, a2337) {
                            Ok(1u64)
                        } else {
                            let a2338 = v2;
                            let a2339 = 79u64;
                            if nat_lt(a2338, a2339) {
                                Ok(6u64)
                            } else {
                                Ok(4u64)
                            }
                        }
                    } else {
                        let a2340 = v2;
                        let a2341 = 82u64;
                        if nat_lt(a2340, a2341) {
                            let a2342 = v2;
                            let a2343 = 81u64;
                            if nat_lt(a2342, a2343) {
                                Ok(9u64)
                            } else {
                                Ok(14u64)
                            }
                        } else {
                            let a2344 = v2;
                            let a2345 = 83u64;
                            if nat_lt(a2344, a2345) {
                                Ok(11u64)
                            } else {
                                Ok(5u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a2346 = v2;
            let a2347 = 98u64;
            if nat_lt(a2346, a2347) {
                let a2348 = v2;
                let a2349 = 91u64;
                if nat_lt(a2348, a2349) {
                    let a2350 = v2;
                    let a2351 = 87u64;
                    if nat_lt(a2350, a2351) {
                        let a2352 = v2;
                        let a2353 = 85u64;
                        if nat_lt(a2352, a2353) {
                            Ok(8u64)
                        } else {
                            let a2354 = v2;
                            let a2355 = 86u64;
                            if nat_lt(a2354, a2355) {
                                Ok(12u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    } else {
                        let a2356 = v2;
                        let a2357 = 89u64;
                        if nat_lt(a2356, a2357) {
                            let a2358 = v2;
                            let a2359 = 88u64;
                            if nat_lt(a2358, a2359) {
                                Ok(1u64)
                            } else {
                                Ok(13u64)
                            }
                        } else {
                            let a2360 = v2;
                            let a2361 = 90u64;
                            if nat_lt(a2360, a2361) {
                                Ok(3u64)
                            } else {
                                Ok(0u64)
                            }
                        }
                    }
                } else {
                    let a2362 = v2;
                    let a2363 = 94u64;
                    if nat_lt(a2362, a2363) {
                        let a2364 = v2;
                        let a2365 = 92u64;
                        if nat_lt(a2364, a2365) {
                            Ok(10u64)
                        } else {
                            let a2366 = v2;
                            let a2367 = 93u64;
                            if nat_lt(a2366, a2367) {
                                Ok(2u64)
                            } else {
                                Ok(6u64)
                            }
                        }
                    } else {
                        let a2368 = v2;
                        let a2369 = 96u64;
                        if nat_lt(a2368, a2369) {
                            let a2370 = v2;
                            let a2371 = 95u64;
                            if nat_lt(a2370, a2371) {
                                Ok(4u64)
                            } else {
                                Ok(7u64)
                            }
                        } else {
                            let a2372 = v2;
                            let a2373 = 97u64;
                            if nat_lt(a2372, a2373) {
                                Ok(11u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    }
                }
            } else {
                let a2374 = v2;
                let a2375 = 105u64;
                if nat_lt(a2374, a2375) {
                    let a2376 = v2;
                    let a2377 = 101u64;
                    if nat_lt(a2376, a2377) {
                        let a2378 = v2;
                        let a2379 = 99u64;
                        if nat_lt(a2378, a2379) {
                            Ok(5u64)
                        } else {
                            let a2380 = v2;
                            let a2381 = 100u64;
                            if nat_lt(a2380, a2381) {
                                Ok(0u64)
                            } else {
                                Ok(1u64)
                            }
                        }
                    } else {
                        let a2382 = v2;
                        let a2383 = 103u64;
                        if nat_lt(a2382, a2383) {
                            let a2384 = v2;
                            let a2385 = 102u64;
                            if nat_lt(a2384, a2385) {
                                Ok(9u64)
                            } else {
                                Ok(8u64)
                            }
                        } else {
                            let a2386 = v2;
                            let a2387 = 104u64;
                            if nat_lt(a2386, a2387) {
                                Ok(6u64)
                            } else {
                                Ok(14u64)
                            }
                        }
                    }
                } else {
                    let a2388 = v2;
                    let a2389 = 108u64;
                    if nat_lt(a2388, a2389) {
                        let a2390 = v2;
                        let a2391 = 106u64;
                        if nat_lt(a2390, a2391) {
                            Ok(10u64)
                        } else {
                            let a2392 = v2;
                            let a2393 = 107u64;
                            if nat_lt(a2392, a2393) {
                                Ok(2u64)
                            } else {
                                Ok(12u64)
                            }
                        }
                    } else {
                        let a2394 = v2;
                        let a2395 = 110u64;
                        if nat_lt(a2394, a2395) {
                            let a2396 = v2;
                            let a2397 = 109u64;
                            if nat_lt(a2396, a2397) {
                                Ok(3u64)
                            } else {
                                Ok(4u64)
                            }
                        } else {
                            let a2398 = v2;
                            let a2399 = 111u64;
                            if nat_lt(a2398, a2399) {
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

pub fn f85(v0: u64) -> u32 {
    let a2400 = v0;
    let a2401 = 12u64;
    if nat_lt(a2400, a2401) {
        let a2402 = v0;
        let a2403 = 9u64;
        if nat_lt(a2402, a2403) {
            let a2404 = v0;
            let a2405 = 8u64;
            if nat_lt(a2404, a2405) {
                7u32
            } else {
                8u32
            }
        } else {
            let a2406 = v0;
            let a2407 = 10u64;
            if nat_lt(a2406, a2407) {
                9u32
            } else {
                let a2408 = v0;
                let a2409 = 11u64;
                if nat_lt(a2408, a2409) {
                    10u32
                } else {
                    11u32
                }
            }
        }
    } else {
        let a2410 = v0;
        let a2411 = 14u64;
        if nat_lt(a2410, a2411) {
            let a2412 = v0;
            let a2413 = 13u64;
            if nat_lt(a2412, a2413) {
                12u32
            } else {
                13u32
            }
        } else {
            let a2414 = v0;
            let a2415 = 15u64;
            if nat_lt(a2414, a2415) {
                14u32
            } else {
                let a2416 = v0;
                let a2417 = 16u64;
                if nat_lt(a2416, a2417) {
                    15u32
                } else {
                    16u32
                }
            }
        }
    }
}

pub fn f86(v0: Adt3, v1: u64) -> u32 {
    let a2418 = v1;
    let a2419 = 4u64;
    if nat_lt(a2418, a2419) {
        let a2420 = v1;
        let a2421 = 2u64;
        if nat_lt(a2420, a2421) {
            let a2422 = v1;
            let a2423 = 1u64;
            if nat_lt(a2422, a2423) {
                match v0 {
                    Adt3::C0(h2424, _, _, _, _, _, _, _) => {
                        h2424
                    }
                }
            } else {
                match v0 {
                    Adt3::C0(_, h2425, _, _, _, _, _, _) => {
                        h2425
                    }
                }
            }
        } else {
            let a2426 = v1;
            let a2427 = 3u64;
            if nat_lt(a2426, a2427) {
                match v0 {
                    Adt3::C0(_, _, h2428, _, _, _, _, _) => {
                        h2428
                    }
                }
            } else {
                match v0 {
                    Adt3::C0(_, _, _, h2429, _, _, _, _) => {
                        h2429
                    }
                }
            }
        }
    } else {
        let a2430 = v1;
        let a2431 = 6u64;
        if nat_lt(a2430, a2431) {
            let a2432 = v1;
            let a2433 = 5u64;
            if nat_lt(a2432, a2433) {
                match v0 {
                    Adt3::C0(_, _, _, _, h2434, _, _, _) => {
                        h2434
                    }
                }
            } else {
                match v0 {
                    Adt3::C0(_, _, _, _, _, h2435, _, _) => {
                        h2435
                    }
                }
            }
        } else {
            let a2436 = v1;
            let a2437 = 7u64;
            if nat_lt(a2436, a2437) {
                match v0 {
                    Adt3::C0(_, _, _, _, _, _, h2438, _) => {
                        h2438
                    }
                }
            } else {
                match v0 {
                    Adt3::C0(_, _, _, _, _, _, _, h2439) => {
                        h2439
                    }
                }
            }
        }
    }
}

pub fn f87(v0: Adt13, v1: u32, v2: u32) -> u32 {
    let m2440 = v0;
    match m2440 {
        Adt13::C0 => {
            f88(v1, v2)
        }
        Adt13::C1 => {
            f88(v1, v2)
        }
        Adt13::C2 => {
            f89(v1, v2)
        }
        Adt13::C3 => {
            f90(v1, v2)
        }
        Adt13::C4 => {
            f89(v1, v2)
        }
    }
}

pub fn f88(v0: u32, v1: u32) -> u32 {
    let m2443 = {
        let a2441 = f91(v0);
        let a2442 = f91(v1);
        fixed_u64::checked_add(a2441, a2442)
    };
    match m2443 {
        None => {
            0u32
        }
        Some(v2) => {
            f92(v2)
        }
    }
}

pub fn f89(v0: u32, v1: u32) -> u32 {
    let a2444 = v0;
    let a2445 = v1;
    fixed_u32::bit_xor(a2444, a2445)
}

pub fn f90(v0: u32, v1: u32) -> u32 {
    let m2448 = {
        let a2446 = 32u32;
        let a2447 = v1;
        fixed_u32::checked_sub(a2446, a2447)
    };
    match m2448 {
        None => {
            v0
        }
        Some(v2) => {
            f93(v0, v1, v2)
        }
    }
}

pub fn f91(v0: u32) -> u64 {
    let m2450 = {
        let a2449 = v0;
        fixed_u64::convert(i128::from(a2449))
    };
    match m2450 {
        None => {
            0u64
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f92(v0: u64) -> u32 {
    let m2454 = {
        let a2453 = {
            let a2451 = v0;
            let a2452 = 4294967295u64;
            fixed_u64::bit_and(a2451, a2452)
        };
        fixed_u32::convert(i128::from(a2453))
    };
    match m2454 {
        None => {
            0u32
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f93(v0: u32, v1: u32, v2: u32) -> u32 {
    let a2455 = f61(v0, v1);
    let a2456 = f73(v0, v2);
    fixed_u32::bit_or(a2455, a2456)
}

pub fn run(p0: Adt0, p1: List<Bytes>, p2: u64) -> R<Result<Bytes, Adt1>> {
    f0(p0, p1, p2)
}
