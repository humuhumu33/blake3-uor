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
    C0(u64, List<Bytes>, u64, List<Bytes>, List<Bytes>, u64, Option<Bytes>, Bytes, List<Bytes>, List<Bytes>, u64),
}

#[derive(Clone)]
pub enum Adt3 {
    C0(Adt4),
    C1(Adt7, u64),
    C2(Adt7, u64),
}

#[derive(Clone)]
pub enum Adt4 {
    C0(u64, u64, u64, u64, u64, Adt5, Adt6, u32, u32, u32, u32, Adt6),
}

#[derive(Clone)]
pub enum Adt5 {
    C0(u32, u32, u32, u32, u32, u32, u32, u32),
}

#[derive(Clone)]
pub enum Adt6 {
    C0(u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32),
}

#[derive(Clone)]
pub enum Adt7 {
    C0(Adt5, u64, u64),
}

#[derive(Clone)]
pub enum Adt8 {
    C0(u64, u64, u64, Adt5, u32, List<Adt3>, List<Adt3>),
}

#[derive(Clone)]
pub enum Adt9 {
    C0(Adt5, u32, u64, Bytes, List<Adt7>, List<Adt3>, List<Adt3>),
}

#[derive(Clone)]
pub enum Adt10 {
    C0(u64, u64, u64, u64, u64, Adt5, Adt6, u64, u32, u32),
}

#[derive(Clone)]
pub enum Adt11 {
    C0(u32, u32, u32, u32, u32, u32, u32, u32),
}

#[derive(Clone)]
pub enum Adt12 {
    C0(Adt7, u64, List<Adt7>, List<Adt3>),
}

#[derive(Clone)]
pub enum Adt13 {
    C0(Adt5, Adt6, u32, u32, u32, u32, List<Adt14>),
}

#[derive(Clone)]
pub enum Adt14 {
    C0(u64, u64, u64, Adt15, u64, u64, u32, u32, u32),
}

#[derive(Clone)]
pub enum Adt15 {
    C0,
    C1,
    C2,
    C3,
    C4,
}

#[derive(Clone)]
pub enum Adt16 {
    C0(Adt6, Adt6, List<Adt14>),
}

#[derive(Clone)]
pub enum Adt17 {
    C0(u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32),
}

#[derive(Clone)]
pub enum Adt18 {
    C0(u32, u32, u32, u32, u32, u32, u32, u32, Adt17, u64),
}

#[derive(Clone)]
pub enum Adt19 {
    C0(u64, u64, Adt15, u64, u64),
}

#[derive(Clone)]
pub enum Fn0 {
    F18,
    F55,
    F78,
}

impl Fn0 {
    pub fn apply(&self, p0: List<Bytes>, p1: Bytes) -> List<Bytes> {
        match self {
            Fn0::F18 => f18(p0, p1),
            Fn0::F55 => f55(p0, p1),
            Fn0::F78 => f78(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn1 {
    F20,
}

impl Fn1 {
    pub fn apply(&self, p0: List<Bytes>) -> Option<List<Bytes>> {
        match self {
            Fn1::F20 => f20(p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn2 {
    F24,
}

impl Fn2 {
    pub fn apply(&self, p0: Adt9, p1: Bytes) -> R<Adt9> {
        match self {
            Fn2::F24 => f24(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn3 {
    F27,
}

impl Fn3 {
    pub fn apply(&self, p0: Adt2, p1: Adt3) -> R<Adt2> {
        match self {
            Fn3::F27 => f27(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn4 {
    F31,
    F143,
}

impl Fn4 {
    pub fn apply(&self, p0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
        match self {
            Fn4::F31 => f31(p0),
            Fn4::F143 => f143(p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn5 {
    F40(u32, Adt5),
}

impl Fn5 {
    pub fn apply(&self, p0: (Adt10, (List<Adt3>, u64)), p1: Adt7) -> R<(Adt10, (List<Adt3>, u64))> {
        match self {
            Fn5::F40(k0, k1) => f40(*k0, k1.clone(), p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn6 {
    F42((Adt10, (List<Adt3>, u64))),
}

impl Fn6 {
    pub fn apply(&self, p0: (u64, List<Adt3>)) -> R<(u64, List<Adt3>)> {
        match self {
            Fn6::F42(k0) => f42(k0.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn7 {
    F54,
}

impl Fn7 {
    pub fn apply(&self, p0: (Option<Bytes>, List<Bytes>), p1: Bytes) -> (Option<Bytes>, List<Bytes>) {
        match self {
            Fn7::F54 => f54(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn8 {
    F63(u32, u64, u64, Adt5, u64, Bytes),
}

impl Fn8 {
    pub fn apply(&self, p0: (u64, (Adt5, List<Adt3>))) -> R<(u64, (Adt5, List<Adt3>))> {
        match self {
            Fn8::F63(k0, k1, k2, k3, k4, k5) => f63(*k0, *k1, *k2, k3.clone(), *k4, k5.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn9 {
    F71,
}

impl Fn9 {
    pub fn apply(&self, p0: List<Adt3>, p1: Adt3) -> List<Adt3> {
        match self {
            Fn9::F71 => f71(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn10 {
    F73(u32, Bytes, Adt5, Adt9),
}

impl Fn10 {
    pub fn apply(&self, p0: (u64, (List<Adt7>, List<Adt3>))) -> R<(u64, (List<Adt7>, List<Adt3>))> {
        match self {
            Fn10::F73(k0, k1, k2, k3) => f73(*k0, k1.clone(), k2.clone(), k3.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn11 {
    F77,
}

impl Fn11 {
    pub fn apply(&self, p0: List<Bytes>, p1: Adt3) -> List<Bytes> {
        match self {
            Fn11::F77 => f77(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn12 {
    F84(Bytes, u64, u64),
}

impl Fn12 {
    pub fn apply(&self, p0: (u64, Adt11)) -> R<(u64, Adt11)> {
        match self {
            Fn12::F84(k0, k1, k2) => f84(k0.clone(), *k1, *k2, p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn13 {
    F88(u64),
}

impl Fn13 {
    pub fn apply(&self, p0: Adt6, p1: Adt3) -> Adt6 {
        match self {
            Fn13::F88(k0) => f88(*k0, p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn14 {
    F98(u64),
}

impl Fn14 {
    pub fn apply(&self, p0: List<Bytes>, p1: Adt14) -> List<Bytes> {
        match self {
            Fn14::F98(k0) => f98(*k0, p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn15 {
    F105(u32, Adt5),
}

impl Fn15 {
    pub fn apply(&self, p0: Adt12) -> R<Option<Adt12>> {
        match self {
            Fn15::F105(k0, k1) => f105(*k0, k1.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn16 {
    F108(Adt5, Adt6),
}

impl Fn16 {
    pub fn apply(&self, p0: (u64, Adt16)) -> R<(u64, Adt16)> {
        match self {
            Fn16::F108(k0, k1) => f108(k0.clone(), k1.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn17 {
    F113,
}

impl Fn17 {
    pub fn apply(&self, p0: Adt6, p1: Adt14) -> Adt6 {
        match self {
            Fn17::F113 => f113(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn18 {
    F115,
}

impl Fn18 {
    pub fn apply(&self, p0: Adt18) -> R<Adt18> {
        match self {
            Fn18::F115 => f115(p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn19 {
    F149(u32, u32),
}

impl Fn19 {
    pub fn apply(&self, p0: (u32, (u32, Bytes))) -> (u32, (u32, Bytes)) {
        match self {
            Fn19::F149(k0, k1) => f149(*k0, *k1, p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn20 {
    F151(u32, u32),
}

impl Fn20 {
    pub fn apply(&self, p0: (u32, Bytes)) -> (u32, Bytes) {
        match self {
            Fn20::F151(k0, k1) => f151(*k0, *k1, p0),
        }
    }
}

pub fn f0(v0: Adt0, v1: List<Bytes>, v2: u64, v3: u64) -> R<Result<Bytes, Adt1>> {
    let m1 = f1(v0, v1, v2, v3)?;
    match m1 {
        Ok(v4) => {
            Ok(Ok::<Bytes, Adt1>({
                let a12 = Bytes::lit(&[0x42, 0x33, 0x54, 0x52, 0x41, 0x43, 0x45, 0x31]);
                let a13 = {
                    let a10 = f2(v3);
                    let a11 = {
                        let a8 = f3({
                            let m4 = {
                                let (h3, _) = v4.clone();
                                h3
                            };
                            match m4 {
                                Adt2::C0(_, _, h2, _, _, _, _, _, _, _, _) => {
                                    h2
                                }
                            }
                        });
                        let a9 = f4({
                            let m7 = {
                                let (h6, _) = v4;
                                h6
                            };
                            match m7 {
                                Adt2::C0(_, h5, _, _, _, _, _, _, _, _, _) => {
                                    h5
                                }
                            }
                        });
                        append_bytes(a8, a9)
                    };
                    append_bytes(a10, a11)
                };
                append_bytes(a12, a13)
            }))
        }
        Err(v5) => {
            Ok(Err::<Bytes, Adt1>(v5))
        }
    }
}

pub fn f1(v0: Adt0, v1: List<Bytes>, v2: u64, v3: u64) -> R<Result<(Adt2, Bytes), Adt1>> {
    let v4: Bytes = f5(v1.clone());
    let m14 = f6(v0.clone(), v1, v2)?;
    match m14 {
        Ok(v5) => {
            Ok(Ok::<(Adt2, Bytes), Adt1>({
                let m15 = v0;
                match m15 {
                    Adt0::C0 => {
                        f7(f8(v3, List::<Bytes>::nil(), 0u64), v5, f11(v4)?, Bytes::lit(&[]))?
                    }
                    Adt0::C1(_) => {
                        f7(f8(v3, List::<Bytes>::nil(), 0u64), v5, f11(v4)?, Bytes::lit(&[]))?
                    }
                    Adt0::C2(v6) => {
                        let v7: (Adt2, Bytes) = f7(f8(v3, List::<Bytes>::nil(), 0u64), Adt8::C0(3u64, {
                            let a16 = v6.clone();
                            length_bytes(a16)
                        }, 32u64, f9(), f10(), List::<Adt3>::nil(), match v5.clone() {
                            Adt8::C0(_, _, _, _, _, h17, _) => {
                                h17
                            }
                        }), f11(v6)?, Bytes::lit(&[]))?;
                        f7({
                            let (h18, _) = v7;
                            h18
                        }, v5, f11(v4)?, {
                            let (_, h19) = v7;
                            h19
                        })?
                    }
                }
            }))
        }
        Err(v9) => {
            Ok(Err::<(Adt2, Bytes), Adt1>(v9))
        }
    }
}

pub fn f2(v0: u64) -> Bytes {
    f12(f13(v0, 8u64))
}

pub fn f3(v0: u64) -> Bytes {
    let a20 = f14(f15(v0));
    let a21 = f14(f16(v0));
    append_bytes(a20, a21)
}

pub fn f4(v0: List<Bytes>) -> Bytes {
    f5(f17(Fn0::F18, List::<Bytes>::nil(), v0))
}

pub fn f5(v0: List<Bytes>) -> Bytes {
    let m23 = {
        let (h22, _) = f19(Fn1::F20, 64u64, v0);
        h22
    };
    match m23.uncons() {
        None => {
            Bytes::lit(&[])
        }
        Some((v1, _)) => {
            v1
        }
    }
}

pub fn f6(v0: Adt0, v1: List<Bytes>, v2: u64) -> R<Result<Adt8, Adt1>> {
    let m24 = f21(v0)?;
    match m24 {
        Ok(v3) => {
            Ok(Ok::<Adt8, Adt1>(f22(f23(Fn2::F24, v3, v1)?, v2)?))
        }
        Err(v4) => {
            Ok(Err::<Adt8, Adt1>(v4))
        }
    }
}

pub fn f7(v0: Adt2, v1: Adt8, v2: Bytes, v3: Bytes) -> R<(Adt2, Bytes)> {
    f25(f26(Fn3::F27, f8(match v0.clone() {
        Adt2::C0(h25, _, _, _, _, _, _, _, _, _, _) => {
            h25
        }
    }, match v0.clone() {
        Adt2::C0(_, h26, _, _, _, _, _, _, _, _, _) => {
            h26
        }
    }, match v0 {
        Adt2::C0(_, _, h27, _, _, _, _, _, _, _, _) => {
            h27
        }
    }), match v1.clone() {
        Adt8::C0(_, _, _, _, _, _, h28) => {
            h28
        }
    })?, v1, v2, v3)
}

pub fn f8(v0: u64, v1: List<Bytes>, v2: u64) -> Adt2 {
    Adt2::C0(v0, v1, v2, List::<Bytes>::nil(), List::<Bytes>::nil(), 0u64, None::<Bytes>, Bytes::lit(&[]), List::<Bytes>::nil(), List::<Bytes>::nil(), 0u64)
}

pub fn f9() -> Adt5 {
    Adt5::C0(1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, 1359893119u32, 2600822924u32, 528734635u32, 1541459225u32)
}

pub fn f10() -> u32 {
    32u32
}

pub fn f11(v0: Bytes) -> R<Bytes> {
    let a29 = f28(v0.clone(), 32u64)?;
    let a30 = f29(v0)?;
    Ok(append_bytes(a29, a30))
}

pub fn f12(v0: u32) -> Bytes {
    let a33 = {
        let a31 = v0;
        let a32 = 128u32;
        fixed_u32::bit_and(a31, a32)
    };
    let a34 = 0u32;
    if equal(a33, a34) {
        let a37 = {
            let a35 = v0;
            let a36 = 64u32;
            fixed_u32::bit_and(a35, a36)
        };
        let a38 = 0u32;
        if equal(a37, a38) {
            let a41 = {
                let a39 = v0;
                let a40 = 32u32;
                fixed_u32::bit_and(a39, a40)
            };
            let a42 = 0u32;
            if equal(a41, a42) {
                let a45 = {
                    let a43 = v0;
                    let a44 = 16u32;
                    fixed_u32::bit_and(a43, a44)
                };
                let a46 = 0u32;
                if equal(a45, a46) {
                    let a49 = {
                        let a47 = v0;
                        let a48 = 8u32;
                        fixed_u32::bit_and(a47, a48)
                    };
                    let a50 = 0u32;
                    if equal(a49, a50) {
                        let a53 = {
                            let a51 = v0;
                            let a52 = 4u32;
                            fixed_u32::bit_and(a51, a52)
                        };
                        let a54 = 0u32;
                        if equal(a53, a54) {
                            let a57 = {
                                let a55 = v0;
                                let a56 = 2u32;
                                fixed_u32::bit_and(a55, a56)
                            };
                            let a58 = 0u32;
                            if equal(a57, a58) {
                                let a61 = {
                                    let a59 = v0;
                                    let a60 = 1u32;
                                    fixed_u32::bit_and(a59, a60)
                                };
                                let a62 = 0u32;
                                if equal(a61, a62) {
                                    Bytes::lit(&[0x00])
                                } else {
                                    Bytes::lit(&[0x01])
                                }
                            } else {
                                let a65 = {
                                    let a63 = v0;
                                    let a64 = 1u32;
                                    fixed_u32::bit_and(a63, a64)
                                };
                                let a66 = 0u32;
                                if equal(a65, a66) {
                                    Bytes::lit(&[0x02])
                                } else {
                                    Bytes::lit(&[0x03])
                                }
                            }
                        } else {
                            let a69 = {
                                let a67 = v0;
                                let a68 = 2u32;
                                fixed_u32::bit_and(a67, a68)
                            };
                            let a70 = 0u32;
                            if equal(a69, a70) {
                                let a73 = {
                                    let a71 = v0;
                                    let a72 = 1u32;
                                    fixed_u32::bit_and(a71, a72)
                                };
                                let a74 = 0u32;
                                if equal(a73, a74) {
                                    Bytes::lit(&[0x04])
                                } else {
                                    Bytes::lit(&[0x05])
                                }
                            } else {
                                let a77 = {
                                    let a75 = v0;
                                    let a76 = 1u32;
                                    fixed_u32::bit_and(a75, a76)
                                };
                                let a78 = 0u32;
                                if equal(a77, a78) {
                                    Bytes::lit(&[0x06])
                                } else {
                                    Bytes::lit(&[0x07])
                                }
                            }
                        }
                    } else {
                        let a81 = {
                            let a79 = v0;
                            let a80 = 4u32;
                            fixed_u32::bit_and(a79, a80)
                        };
                        let a82 = 0u32;
                        if equal(a81, a82) {
                            let a85 = {
                                let a83 = v0;
                                let a84 = 2u32;
                                fixed_u32::bit_and(a83, a84)
                            };
                            let a86 = 0u32;
                            if equal(a85, a86) {
                                let a89 = {
                                    let a87 = v0;
                                    let a88 = 1u32;
                                    fixed_u32::bit_and(a87, a88)
                                };
                                let a90 = 0u32;
                                if equal(a89, a90) {
                                    Bytes::lit(&[0x08])
                                } else {
                                    Bytes::lit(&[0x09])
                                }
                            } else {
                                let a93 = {
                                    let a91 = v0;
                                    let a92 = 1u32;
                                    fixed_u32::bit_and(a91, a92)
                                };
                                let a94 = 0u32;
                                if equal(a93, a94) {
                                    Bytes::lit(&[0x0a])
                                } else {
                                    Bytes::lit(&[0x0b])
                                }
                            }
                        } else {
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
                                    Bytes::lit(&[0x0c])
                                } else {
                                    Bytes::lit(&[0x0d])
                                }
                            } else {
                                let a105 = {
                                    let a103 = v0;
                                    let a104 = 1u32;
                                    fixed_u32::bit_and(a103, a104)
                                };
                                let a106 = 0u32;
                                if equal(a105, a106) {
                                    Bytes::lit(&[0x0e])
                                } else {
                                    Bytes::lit(&[0x0f])
                                }
                            }
                        }
                    }
                } else {
                    let a109 = {
                        let a107 = v0;
                        let a108 = 8u32;
                        fixed_u32::bit_and(a107, a108)
                    };
                    let a110 = 0u32;
                    if equal(a109, a110) {
                        let a113 = {
                            let a111 = v0;
                            let a112 = 4u32;
                            fixed_u32::bit_and(a111, a112)
                        };
                        let a114 = 0u32;
                        if equal(a113, a114) {
                            let a117 = {
                                let a115 = v0;
                                let a116 = 2u32;
                                fixed_u32::bit_and(a115, a116)
                            };
                            let a118 = 0u32;
                            if equal(a117, a118) {
                                let a121 = {
                                    let a119 = v0;
                                    let a120 = 1u32;
                                    fixed_u32::bit_and(a119, a120)
                                };
                                let a122 = 0u32;
                                if equal(a121, a122) {
                                    Bytes::lit(&[0x10])
                                } else {
                                    Bytes::lit(&[0x11])
                                }
                            } else {
                                let a125 = {
                                    let a123 = v0;
                                    let a124 = 1u32;
                                    fixed_u32::bit_and(a123, a124)
                                };
                                let a126 = 0u32;
                                if equal(a125, a126) {
                                    Bytes::lit(&[0x12])
                                } else {
                                    Bytes::lit(&[0x13])
                                }
                            }
                        } else {
                            let a129 = {
                                let a127 = v0;
                                let a128 = 2u32;
                                fixed_u32::bit_and(a127, a128)
                            };
                            let a130 = 0u32;
                            if equal(a129, a130) {
                                let a133 = {
                                    let a131 = v0;
                                    let a132 = 1u32;
                                    fixed_u32::bit_and(a131, a132)
                                };
                                let a134 = 0u32;
                                if equal(a133, a134) {
                                    Bytes::lit(&[0x14])
                                } else {
                                    Bytes::lit(&[0x15])
                                }
                            } else {
                                let a137 = {
                                    let a135 = v0;
                                    let a136 = 1u32;
                                    fixed_u32::bit_and(a135, a136)
                                };
                                let a138 = 0u32;
                                if equal(a137, a138) {
                                    Bytes::lit(&[0x16])
                                } else {
                                    Bytes::lit(&[0x17])
                                }
                            }
                        }
                    } else {
                        let a141 = {
                            let a139 = v0;
                            let a140 = 4u32;
                            fixed_u32::bit_and(a139, a140)
                        };
                        let a142 = 0u32;
                        if equal(a141, a142) {
                            let a145 = {
                                let a143 = v0;
                                let a144 = 2u32;
                                fixed_u32::bit_and(a143, a144)
                            };
                            let a146 = 0u32;
                            if equal(a145, a146) {
                                let a149 = {
                                    let a147 = v0;
                                    let a148 = 1u32;
                                    fixed_u32::bit_and(a147, a148)
                                };
                                let a150 = 0u32;
                                if equal(a149, a150) {
                                    Bytes::lit(&[0x18])
                                } else {
                                    Bytes::lit(&[0x19])
                                }
                            } else {
                                let a153 = {
                                    let a151 = v0;
                                    let a152 = 1u32;
                                    fixed_u32::bit_and(a151, a152)
                                };
                                let a154 = 0u32;
                                if equal(a153, a154) {
                                    Bytes::lit(&[0x1a])
                                } else {
                                    Bytes::lit(&[0x1b])
                                }
                            }
                        } else {
                            let a157 = {
                                let a155 = v0;
                                let a156 = 2u32;
                                fixed_u32::bit_and(a155, a156)
                            };
                            let a158 = 0u32;
                            if equal(a157, a158) {
                                let a161 = {
                                    let a159 = v0;
                                    let a160 = 1u32;
                                    fixed_u32::bit_and(a159, a160)
                                };
                                let a162 = 0u32;
                                if equal(a161, a162) {
                                    Bytes::lit(&[0x1c])
                                } else {
                                    Bytes::lit(&[0x1d])
                                }
                            } else {
                                let a165 = {
                                    let a163 = v0;
                                    let a164 = 1u32;
                                    fixed_u32::bit_and(a163, a164)
                                };
                                let a166 = 0u32;
                                if equal(a165, a166) {
                                    Bytes::lit(&[0x1e])
                                } else {
                                    Bytes::lit(&[0x1f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a169 = {
                    let a167 = v0;
                    let a168 = 16u32;
                    fixed_u32::bit_and(a167, a168)
                };
                let a170 = 0u32;
                if equal(a169, a170) {
                    let a173 = {
                        let a171 = v0;
                        let a172 = 8u32;
                        fixed_u32::bit_and(a171, a172)
                    };
                    let a174 = 0u32;
                    if equal(a173, a174) {
                        let a177 = {
                            let a175 = v0;
                            let a176 = 4u32;
                            fixed_u32::bit_and(a175, a176)
                        };
                        let a178 = 0u32;
                        if equal(a177, a178) {
                            let a181 = {
                                let a179 = v0;
                                let a180 = 2u32;
                                fixed_u32::bit_and(a179, a180)
                            };
                            let a182 = 0u32;
                            if equal(a181, a182) {
                                let a185 = {
                                    let a183 = v0;
                                    let a184 = 1u32;
                                    fixed_u32::bit_and(a183, a184)
                                };
                                let a186 = 0u32;
                                if equal(a185, a186) {
                                    Bytes::lit(&[0x20])
                                } else {
                                    Bytes::lit(&[0x21])
                                }
                            } else {
                                let a189 = {
                                    let a187 = v0;
                                    let a188 = 1u32;
                                    fixed_u32::bit_and(a187, a188)
                                };
                                let a190 = 0u32;
                                if equal(a189, a190) {
                                    Bytes::lit(&[0x22])
                                } else {
                                    Bytes::lit(&[0x23])
                                }
                            }
                        } else {
                            let a193 = {
                                let a191 = v0;
                                let a192 = 2u32;
                                fixed_u32::bit_and(a191, a192)
                            };
                            let a194 = 0u32;
                            if equal(a193, a194) {
                                let a197 = {
                                    let a195 = v0;
                                    let a196 = 1u32;
                                    fixed_u32::bit_and(a195, a196)
                                };
                                let a198 = 0u32;
                                if equal(a197, a198) {
                                    Bytes::lit(&[0x24])
                                } else {
                                    Bytes::lit(&[0x25])
                                }
                            } else {
                                let a201 = {
                                    let a199 = v0;
                                    let a200 = 1u32;
                                    fixed_u32::bit_and(a199, a200)
                                };
                                let a202 = 0u32;
                                if equal(a201, a202) {
                                    Bytes::lit(&[0x26])
                                } else {
                                    Bytes::lit(&[0x27])
                                }
                            }
                        }
                    } else {
                        let a205 = {
                            let a203 = v0;
                            let a204 = 4u32;
                            fixed_u32::bit_and(a203, a204)
                        };
                        let a206 = 0u32;
                        if equal(a205, a206) {
                            let a209 = {
                                let a207 = v0;
                                let a208 = 2u32;
                                fixed_u32::bit_and(a207, a208)
                            };
                            let a210 = 0u32;
                            if equal(a209, a210) {
                                let a213 = {
                                    let a211 = v0;
                                    let a212 = 1u32;
                                    fixed_u32::bit_and(a211, a212)
                                };
                                let a214 = 0u32;
                                if equal(a213, a214) {
                                    Bytes::lit(&[0x28])
                                } else {
                                    Bytes::lit(&[0x29])
                                }
                            } else {
                                let a217 = {
                                    let a215 = v0;
                                    let a216 = 1u32;
                                    fixed_u32::bit_and(a215, a216)
                                };
                                let a218 = 0u32;
                                if equal(a217, a218) {
                                    Bytes::lit(&[0x2a])
                                } else {
                                    Bytes::lit(&[0x2b])
                                }
                            }
                        } else {
                            let a221 = {
                                let a219 = v0;
                                let a220 = 2u32;
                                fixed_u32::bit_and(a219, a220)
                            };
                            let a222 = 0u32;
                            if equal(a221, a222) {
                                let a225 = {
                                    let a223 = v0;
                                    let a224 = 1u32;
                                    fixed_u32::bit_and(a223, a224)
                                };
                                let a226 = 0u32;
                                if equal(a225, a226) {
                                    Bytes::lit(&[0x2c])
                                } else {
                                    Bytes::lit(&[0x2d])
                                }
                            } else {
                                let a229 = {
                                    let a227 = v0;
                                    let a228 = 1u32;
                                    fixed_u32::bit_and(a227, a228)
                                };
                                let a230 = 0u32;
                                if equal(a229, a230) {
                                    Bytes::lit(&[0x2e])
                                } else {
                                    Bytes::lit(&[0x2f])
                                }
                            }
                        }
                    }
                } else {
                    let a233 = {
                        let a231 = v0;
                        let a232 = 8u32;
                        fixed_u32::bit_and(a231, a232)
                    };
                    let a234 = 0u32;
                    if equal(a233, a234) {
                        let a237 = {
                            let a235 = v0;
                            let a236 = 4u32;
                            fixed_u32::bit_and(a235, a236)
                        };
                        let a238 = 0u32;
                        if equal(a237, a238) {
                            let a241 = {
                                let a239 = v0;
                                let a240 = 2u32;
                                fixed_u32::bit_and(a239, a240)
                            };
                            let a242 = 0u32;
                            if equal(a241, a242) {
                                let a245 = {
                                    let a243 = v0;
                                    let a244 = 1u32;
                                    fixed_u32::bit_and(a243, a244)
                                };
                                let a246 = 0u32;
                                if equal(a245, a246) {
                                    Bytes::lit(&[0x30])
                                } else {
                                    Bytes::lit(&[0x31])
                                }
                            } else {
                                let a249 = {
                                    let a247 = v0;
                                    let a248 = 1u32;
                                    fixed_u32::bit_and(a247, a248)
                                };
                                let a250 = 0u32;
                                if equal(a249, a250) {
                                    Bytes::lit(&[0x32])
                                } else {
                                    Bytes::lit(&[0x33])
                                }
                            }
                        } else {
                            let a253 = {
                                let a251 = v0;
                                let a252 = 2u32;
                                fixed_u32::bit_and(a251, a252)
                            };
                            let a254 = 0u32;
                            if equal(a253, a254) {
                                let a257 = {
                                    let a255 = v0;
                                    let a256 = 1u32;
                                    fixed_u32::bit_and(a255, a256)
                                };
                                let a258 = 0u32;
                                if equal(a257, a258) {
                                    Bytes::lit(&[0x34])
                                } else {
                                    Bytes::lit(&[0x35])
                                }
                            } else {
                                let a261 = {
                                    let a259 = v0;
                                    let a260 = 1u32;
                                    fixed_u32::bit_and(a259, a260)
                                };
                                let a262 = 0u32;
                                if equal(a261, a262) {
                                    Bytes::lit(&[0x36])
                                } else {
                                    Bytes::lit(&[0x37])
                                }
                            }
                        }
                    } else {
                        let a265 = {
                            let a263 = v0;
                            let a264 = 4u32;
                            fixed_u32::bit_and(a263, a264)
                        };
                        let a266 = 0u32;
                        if equal(a265, a266) {
                            let a269 = {
                                let a267 = v0;
                                let a268 = 2u32;
                                fixed_u32::bit_and(a267, a268)
                            };
                            let a270 = 0u32;
                            if equal(a269, a270) {
                                let a273 = {
                                    let a271 = v0;
                                    let a272 = 1u32;
                                    fixed_u32::bit_and(a271, a272)
                                };
                                let a274 = 0u32;
                                if equal(a273, a274) {
                                    Bytes::lit(&[0x38])
                                } else {
                                    Bytes::lit(&[0x39])
                                }
                            } else {
                                let a277 = {
                                    let a275 = v0;
                                    let a276 = 1u32;
                                    fixed_u32::bit_and(a275, a276)
                                };
                                let a278 = 0u32;
                                if equal(a277, a278) {
                                    Bytes::lit(&[0x3a])
                                } else {
                                    Bytes::lit(&[0x3b])
                                }
                            }
                        } else {
                            let a281 = {
                                let a279 = v0;
                                let a280 = 2u32;
                                fixed_u32::bit_and(a279, a280)
                            };
                            let a282 = 0u32;
                            if equal(a281, a282) {
                                let a285 = {
                                    let a283 = v0;
                                    let a284 = 1u32;
                                    fixed_u32::bit_and(a283, a284)
                                };
                                let a286 = 0u32;
                                if equal(a285, a286) {
                                    Bytes::lit(&[0x3c])
                                } else {
                                    Bytes::lit(&[0x3d])
                                }
                            } else {
                                let a289 = {
                                    let a287 = v0;
                                    let a288 = 1u32;
                                    fixed_u32::bit_and(a287, a288)
                                };
                                let a290 = 0u32;
                                if equal(a289, a290) {
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
            let a293 = {
                let a291 = v0;
                let a292 = 32u32;
                fixed_u32::bit_and(a291, a292)
            };
            let a294 = 0u32;
            if equal(a293, a294) {
                let a297 = {
                    let a295 = v0;
                    let a296 = 16u32;
                    fixed_u32::bit_and(a295, a296)
                };
                let a298 = 0u32;
                if equal(a297, a298) {
                    let a301 = {
                        let a299 = v0;
                        let a300 = 8u32;
                        fixed_u32::bit_and(a299, a300)
                    };
                    let a302 = 0u32;
                    if equal(a301, a302) {
                        let a305 = {
                            let a303 = v0;
                            let a304 = 4u32;
                            fixed_u32::bit_and(a303, a304)
                        };
                        let a306 = 0u32;
                        if equal(a305, a306) {
                            let a309 = {
                                let a307 = v0;
                                let a308 = 2u32;
                                fixed_u32::bit_and(a307, a308)
                            };
                            let a310 = 0u32;
                            if equal(a309, a310) {
                                let a313 = {
                                    let a311 = v0;
                                    let a312 = 1u32;
                                    fixed_u32::bit_and(a311, a312)
                                };
                                let a314 = 0u32;
                                if equal(a313, a314) {
                                    Bytes::lit(&[0x40])
                                } else {
                                    Bytes::lit(&[0x41])
                                }
                            } else {
                                let a317 = {
                                    let a315 = v0;
                                    let a316 = 1u32;
                                    fixed_u32::bit_and(a315, a316)
                                };
                                let a318 = 0u32;
                                if equal(a317, a318) {
                                    Bytes::lit(&[0x42])
                                } else {
                                    Bytes::lit(&[0x43])
                                }
                            }
                        } else {
                            let a321 = {
                                let a319 = v0;
                                let a320 = 2u32;
                                fixed_u32::bit_and(a319, a320)
                            };
                            let a322 = 0u32;
                            if equal(a321, a322) {
                                let a325 = {
                                    let a323 = v0;
                                    let a324 = 1u32;
                                    fixed_u32::bit_and(a323, a324)
                                };
                                let a326 = 0u32;
                                if equal(a325, a326) {
                                    Bytes::lit(&[0x44])
                                } else {
                                    Bytes::lit(&[0x45])
                                }
                            } else {
                                let a329 = {
                                    let a327 = v0;
                                    let a328 = 1u32;
                                    fixed_u32::bit_and(a327, a328)
                                };
                                let a330 = 0u32;
                                if equal(a329, a330) {
                                    Bytes::lit(&[0x46])
                                } else {
                                    Bytes::lit(&[0x47])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0x48])
                                } else {
                                    Bytes::lit(&[0x49])
                                }
                            } else {
                                let a345 = {
                                    let a343 = v0;
                                    let a344 = 1u32;
                                    fixed_u32::bit_and(a343, a344)
                                };
                                let a346 = 0u32;
                                if equal(a345, a346) {
                                    Bytes::lit(&[0x4a])
                                } else {
                                    Bytes::lit(&[0x4b])
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
                                    Bytes::lit(&[0x4c])
                                } else {
                                    Bytes::lit(&[0x4d])
                                }
                            } else {
                                let a357 = {
                                    let a355 = v0;
                                    let a356 = 1u32;
                                    fixed_u32::bit_and(a355, a356)
                                };
                                let a358 = 0u32;
                                if equal(a357, a358) {
                                    Bytes::lit(&[0x4e])
                                } else {
                                    Bytes::lit(&[0x4f])
                                }
                            }
                        }
                    }
                } else {
                    let a361 = {
                        let a359 = v0;
                        let a360 = 8u32;
                        fixed_u32::bit_and(a359, a360)
                    };
                    let a362 = 0u32;
                    if equal(a361, a362) {
                        let a365 = {
                            let a363 = v0;
                            let a364 = 4u32;
                            fixed_u32::bit_and(a363, a364)
                        };
                        let a366 = 0u32;
                        if equal(a365, a366) {
                            let a369 = {
                                let a367 = v0;
                                let a368 = 2u32;
                                fixed_u32::bit_and(a367, a368)
                            };
                            let a370 = 0u32;
                            if equal(a369, a370) {
                                let a373 = {
                                    let a371 = v0;
                                    let a372 = 1u32;
                                    fixed_u32::bit_and(a371, a372)
                                };
                                let a374 = 0u32;
                                if equal(a373, a374) {
                                    Bytes::lit(&[0x50])
                                } else {
                                    Bytes::lit(&[0x51])
                                }
                            } else {
                                let a377 = {
                                    let a375 = v0;
                                    let a376 = 1u32;
                                    fixed_u32::bit_and(a375, a376)
                                };
                                let a378 = 0u32;
                                if equal(a377, a378) {
                                    Bytes::lit(&[0x52])
                                } else {
                                    Bytes::lit(&[0x53])
                                }
                            }
                        } else {
                            let a381 = {
                                let a379 = v0;
                                let a380 = 2u32;
                                fixed_u32::bit_and(a379, a380)
                            };
                            let a382 = 0u32;
                            if equal(a381, a382) {
                                let a385 = {
                                    let a383 = v0;
                                    let a384 = 1u32;
                                    fixed_u32::bit_and(a383, a384)
                                };
                                let a386 = 0u32;
                                if equal(a385, a386) {
                                    Bytes::lit(&[0x54])
                                } else {
                                    Bytes::lit(&[0x55])
                                }
                            } else {
                                let a389 = {
                                    let a387 = v0;
                                    let a388 = 1u32;
                                    fixed_u32::bit_and(a387, a388)
                                };
                                let a390 = 0u32;
                                if equal(a389, a390) {
                                    Bytes::lit(&[0x56])
                                } else {
                                    Bytes::lit(&[0x57])
                                }
                            }
                        }
                    } else {
                        let a393 = {
                            let a391 = v0;
                            let a392 = 4u32;
                            fixed_u32::bit_and(a391, a392)
                        };
                        let a394 = 0u32;
                        if equal(a393, a394) {
                            let a397 = {
                                let a395 = v0;
                                let a396 = 2u32;
                                fixed_u32::bit_and(a395, a396)
                            };
                            let a398 = 0u32;
                            if equal(a397, a398) {
                                let a401 = {
                                    let a399 = v0;
                                    let a400 = 1u32;
                                    fixed_u32::bit_and(a399, a400)
                                };
                                let a402 = 0u32;
                                if equal(a401, a402) {
                                    Bytes::lit(&[0x58])
                                } else {
                                    Bytes::lit(&[0x59])
                                }
                            } else {
                                let a405 = {
                                    let a403 = v0;
                                    let a404 = 1u32;
                                    fixed_u32::bit_and(a403, a404)
                                };
                                let a406 = 0u32;
                                if equal(a405, a406) {
                                    Bytes::lit(&[0x5a])
                                } else {
                                    Bytes::lit(&[0x5b])
                                }
                            }
                        } else {
                            let a409 = {
                                let a407 = v0;
                                let a408 = 2u32;
                                fixed_u32::bit_and(a407, a408)
                            };
                            let a410 = 0u32;
                            if equal(a409, a410) {
                                let a413 = {
                                    let a411 = v0;
                                    let a412 = 1u32;
                                    fixed_u32::bit_and(a411, a412)
                                };
                                let a414 = 0u32;
                                if equal(a413, a414) {
                                    Bytes::lit(&[0x5c])
                                } else {
                                    Bytes::lit(&[0x5d])
                                }
                            } else {
                                let a417 = {
                                    let a415 = v0;
                                    let a416 = 1u32;
                                    fixed_u32::bit_and(a415, a416)
                                };
                                let a418 = 0u32;
                                if equal(a417, a418) {
                                    Bytes::lit(&[0x5e])
                                } else {
                                    Bytes::lit(&[0x5f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a421 = {
                    let a419 = v0;
                    let a420 = 16u32;
                    fixed_u32::bit_and(a419, a420)
                };
                let a422 = 0u32;
                if equal(a421, a422) {
                    let a425 = {
                        let a423 = v0;
                        let a424 = 8u32;
                        fixed_u32::bit_and(a423, a424)
                    };
                    let a426 = 0u32;
                    if equal(a425, a426) {
                        let a429 = {
                            let a427 = v0;
                            let a428 = 4u32;
                            fixed_u32::bit_and(a427, a428)
                        };
                        let a430 = 0u32;
                        if equal(a429, a430) {
                            let a433 = {
                                let a431 = v0;
                                let a432 = 2u32;
                                fixed_u32::bit_and(a431, a432)
                            };
                            let a434 = 0u32;
                            if equal(a433, a434) {
                                let a437 = {
                                    let a435 = v0;
                                    let a436 = 1u32;
                                    fixed_u32::bit_and(a435, a436)
                                };
                                let a438 = 0u32;
                                if equal(a437, a438) {
                                    Bytes::lit(&[0x60])
                                } else {
                                    Bytes::lit(&[0x61])
                                }
                            } else {
                                let a441 = {
                                    let a439 = v0;
                                    let a440 = 1u32;
                                    fixed_u32::bit_and(a439, a440)
                                };
                                let a442 = 0u32;
                                if equal(a441, a442) {
                                    Bytes::lit(&[0x62])
                                } else {
                                    Bytes::lit(&[0x63])
                                }
                            }
                        } else {
                            let a445 = {
                                let a443 = v0;
                                let a444 = 2u32;
                                fixed_u32::bit_and(a443, a444)
                            };
                            let a446 = 0u32;
                            if equal(a445, a446) {
                                let a449 = {
                                    let a447 = v0;
                                    let a448 = 1u32;
                                    fixed_u32::bit_and(a447, a448)
                                };
                                let a450 = 0u32;
                                if equal(a449, a450) {
                                    Bytes::lit(&[0x64])
                                } else {
                                    Bytes::lit(&[0x65])
                                }
                            } else {
                                let a453 = {
                                    let a451 = v0;
                                    let a452 = 1u32;
                                    fixed_u32::bit_and(a451, a452)
                                };
                                let a454 = 0u32;
                                if equal(a453, a454) {
                                    Bytes::lit(&[0x66])
                                } else {
                                    Bytes::lit(&[0x67])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0x68])
                                } else {
                                    Bytes::lit(&[0x69])
                                }
                            } else {
                                let a469 = {
                                    let a467 = v0;
                                    let a468 = 1u32;
                                    fixed_u32::bit_and(a467, a468)
                                };
                                let a470 = 0u32;
                                if equal(a469, a470) {
                                    Bytes::lit(&[0x6a])
                                } else {
                                    Bytes::lit(&[0x6b])
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
                                    Bytes::lit(&[0x6c])
                                } else {
                                    Bytes::lit(&[0x6d])
                                }
                            } else {
                                let a481 = {
                                    let a479 = v0;
                                    let a480 = 1u32;
                                    fixed_u32::bit_and(a479, a480)
                                };
                                let a482 = 0u32;
                                if equal(a481, a482) {
                                    Bytes::lit(&[0x6e])
                                } else {
                                    Bytes::lit(&[0x6f])
                                }
                            }
                        }
                    }
                } else {
                    let a485 = {
                        let a483 = v0;
                        let a484 = 8u32;
                        fixed_u32::bit_and(a483, a484)
                    };
                    let a486 = 0u32;
                    if equal(a485, a486) {
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
                                    Bytes::lit(&[0x70])
                                } else {
                                    Bytes::lit(&[0x71])
                                }
                            } else {
                                let a501 = {
                                    let a499 = v0;
                                    let a500 = 1u32;
                                    fixed_u32::bit_and(a499, a500)
                                };
                                let a502 = 0u32;
                                if equal(a501, a502) {
                                    Bytes::lit(&[0x72])
                                } else {
                                    Bytes::lit(&[0x73])
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
                                    Bytes::lit(&[0x74])
                                } else {
                                    Bytes::lit(&[0x75])
                                }
                            } else {
                                let a513 = {
                                    let a511 = v0;
                                    let a512 = 1u32;
                                    fixed_u32::bit_and(a511, a512)
                                };
                                let a514 = 0u32;
                                if equal(a513, a514) {
                                    Bytes::lit(&[0x76])
                                } else {
                                    Bytes::lit(&[0x77])
                                }
                            }
                        }
                    } else {
                        let a517 = {
                            let a515 = v0;
                            let a516 = 4u32;
                            fixed_u32::bit_and(a515, a516)
                        };
                        let a518 = 0u32;
                        if equal(a517, a518) {
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
                                    Bytes::lit(&[0x78])
                                } else {
                                    Bytes::lit(&[0x79])
                                }
                            } else {
                                let a529 = {
                                    let a527 = v0;
                                    let a528 = 1u32;
                                    fixed_u32::bit_and(a527, a528)
                                };
                                let a530 = 0u32;
                                if equal(a529, a530) {
                                    Bytes::lit(&[0x7a])
                                } else {
                                    Bytes::lit(&[0x7b])
                                }
                            }
                        } else {
                            let a533 = {
                                let a531 = v0;
                                let a532 = 2u32;
                                fixed_u32::bit_and(a531, a532)
                            };
                            let a534 = 0u32;
                            if equal(a533, a534) {
                                let a537 = {
                                    let a535 = v0;
                                    let a536 = 1u32;
                                    fixed_u32::bit_and(a535, a536)
                                };
                                let a538 = 0u32;
                                if equal(a537, a538) {
                                    Bytes::lit(&[0x7c])
                                } else {
                                    Bytes::lit(&[0x7d])
                                }
                            } else {
                                let a541 = {
                                    let a539 = v0;
                                    let a540 = 1u32;
                                    fixed_u32::bit_and(a539, a540)
                                };
                                let a542 = 0u32;
                                if equal(a541, a542) {
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
        let a545 = {
            let a543 = v0;
            let a544 = 64u32;
            fixed_u32::bit_and(a543, a544)
        };
        let a546 = 0u32;
        if equal(a545, a546) {
            let a549 = {
                let a547 = v0;
                let a548 = 32u32;
                fixed_u32::bit_and(a547, a548)
            };
            let a550 = 0u32;
            if equal(a549, a550) {
                let a553 = {
                    let a551 = v0;
                    let a552 = 16u32;
                    fixed_u32::bit_and(a551, a552)
                };
                let a554 = 0u32;
                if equal(a553, a554) {
                    let a557 = {
                        let a555 = v0;
                        let a556 = 8u32;
                        fixed_u32::bit_and(a555, a556)
                    };
                    let a558 = 0u32;
                    if equal(a557, a558) {
                        let a561 = {
                            let a559 = v0;
                            let a560 = 4u32;
                            fixed_u32::bit_and(a559, a560)
                        };
                        let a562 = 0u32;
                        if equal(a561, a562) {
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
                                    Bytes::lit(&[0x80])
                                } else {
                                    Bytes::lit(&[0x81])
                                }
                            } else {
                                let a573 = {
                                    let a571 = v0;
                                    let a572 = 1u32;
                                    fixed_u32::bit_and(a571, a572)
                                };
                                let a574 = 0u32;
                                if equal(a573, a574) {
                                    Bytes::lit(&[0x82])
                                } else {
                                    Bytes::lit(&[0x83])
                                }
                            }
                        } else {
                            let a577 = {
                                let a575 = v0;
                                let a576 = 2u32;
                                fixed_u32::bit_and(a575, a576)
                            };
                            let a578 = 0u32;
                            if equal(a577, a578) {
                                let a581 = {
                                    let a579 = v0;
                                    let a580 = 1u32;
                                    fixed_u32::bit_and(a579, a580)
                                };
                                let a582 = 0u32;
                                if equal(a581, a582) {
                                    Bytes::lit(&[0x84])
                                } else {
                                    Bytes::lit(&[0x85])
                                }
                            } else {
                                let a585 = {
                                    let a583 = v0;
                                    let a584 = 1u32;
                                    fixed_u32::bit_and(a583, a584)
                                };
                                let a586 = 0u32;
                                if equal(a585, a586) {
                                    Bytes::lit(&[0x86])
                                } else {
                                    Bytes::lit(&[0x87])
                                }
                            }
                        }
                    } else {
                        let a589 = {
                            let a587 = v0;
                            let a588 = 4u32;
                            fixed_u32::bit_and(a587, a588)
                        };
                        let a590 = 0u32;
                        if equal(a589, a590) {
                            let a593 = {
                                let a591 = v0;
                                let a592 = 2u32;
                                fixed_u32::bit_and(a591, a592)
                            };
                            let a594 = 0u32;
                            if equal(a593, a594) {
                                let a597 = {
                                    let a595 = v0;
                                    let a596 = 1u32;
                                    fixed_u32::bit_and(a595, a596)
                                };
                                let a598 = 0u32;
                                if equal(a597, a598) {
                                    Bytes::lit(&[0x88])
                                } else {
                                    Bytes::lit(&[0x89])
                                }
                            } else {
                                let a601 = {
                                    let a599 = v0;
                                    let a600 = 1u32;
                                    fixed_u32::bit_and(a599, a600)
                                };
                                let a602 = 0u32;
                                if equal(a601, a602) {
                                    Bytes::lit(&[0x8a])
                                } else {
                                    Bytes::lit(&[0x8b])
                                }
                            }
                        } else {
                            let a605 = {
                                let a603 = v0;
                                let a604 = 2u32;
                                fixed_u32::bit_and(a603, a604)
                            };
                            let a606 = 0u32;
                            if equal(a605, a606) {
                                let a609 = {
                                    let a607 = v0;
                                    let a608 = 1u32;
                                    fixed_u32::bit_and(a607, a608)
                                };
                                let a610 = 0u32;
                                if equal(a609, a610) {
                                    Bytes::lit(&[0x8c])
                                } else {
                                    Bytes::lit(&[0x8d])
                                }
                            } else {
                                let a613 = {
                                    let a611 = v0;
                                    let a612 = 1u32;
                                    fixed_u32::bit_and(a611, a612)
                                };
                                let a614 = 0u32;
                                if equal(a613, a614) {
                                    Bytes::lit(&[0x8e])
                                } else {
                                    Bytes::lit(&[0x8f])
                                }
                            }
                        }
                    }
                } else {
                    let a617 = {
                        let a615 = v0;
                        let a616 = 8u32;
                        fixed_u32::bit_and(a615, a616)
                    };
                    let a618 = 0u32;
                    if equal(a617, a618) {
                        let a621 = {
                            let a619 = v0;
                            let a620 = 4u32;
                            fixed_u32::bit_and(a619, a620)
                        };
                        let a622 = 0u32;
                        if equal(a621, a622) {
                            let a625 = {
                                let a623 = v0;
                                let a624 = 2u32;
                                fixed_u32::bit_and(a623, a624)
                            };
                            let a626 = 0u32;
                            if equal(a625, a626) {
                                let a629 = {
                                    let a627 = v0;
                                    let a628 = 1u32;
                                    fixed_u32::bit_and(a627, a628)
                                };
                                let a630 = 0u32;
                                if equal(a629, a630) {
                                    Bytes::lit(&[0x90])
                                } else {
                                    Bytes::lit(&[0x91])
                                }
                            } else {
                                let a633 = {
                                    let a631 = v0;
                                    let a632 = 1u32;
                                    fixed_u32::bit_and(a631, a632)
                                };
                                let a634 = 0u32;
                                if equal(a633, a634) {
                                    Bytes::lit(&[0x92])
                                } else {
                                    Bytes::lit(&[0x93])
                                }
                            }
                        } else {
                            let a637 = {
                                let a635 = v0;
                                let a636 = 2u32;
                                fixed_u32::bit_and(a635, a636)
                            };
                            let a638 = 0u32;
                            if equal(a637, a638) {
                                let a641 = {
                                    let a639 = v0;
                                    let a640 = 1u32;
                                    fixed_u32::bit_and(a639, a640)
                                };
                                let a642 = 0u32;
                                if equal(a641, a642) {
                                    Bytes::lit(&[0x94])
                                } else {
                                    Bytes::lit(&[0x95])
                                }
                            } else {
                                let a645 = {
                                    let a643 = v0;
                                    let a644 = 1u32;
                                    fixed_u32::bit_and(a643, a644)
                                };
                                let a646 = 0u32;
                                if equal(a645, a646) {
                                    Bytes::lit(&[0x96])
                                } else {
                                    Bytes::lit(&[0x97])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0x98])
                                } else {
                                    Bytes::lit(&[0x99])
                                }
                            } else {
                                let a661 = {
                                    let a659 = v0;
                                    let a660 = 1u32;
                                    fixed_u32::bit_and(a659, a660)
                                };
                                let a662 = 0u32;
                                if equal(a661, a662) {
                                    Bytes::lit(&[0x9a])
                                } else {
                                    Bytes::lit(&[0x9b])
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
                                    Bytes::lit(&[0x9c])
                                } else {
                                    Bytes::lit(&[0x9d])
                                }
                            } else {
                                let a673 = {
                                    let a671 = v0;
                                    let a672 = 1u32;
                                    fixed_u32::bit_and(a671, a672)
                                };
                                let a674 = 0u32;
                                if equal(a673, a674) {
                                    Bytes::lit(&[0x9e])
                                } else {
                                    Bytes::lit(&[0x9f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a677 = {
                    let a675 = v0;
                    let a676 = 16u32;
                    fixed_u32::bit_and(a675, a676)
                };
                let a678 = 0u32;
                if equal(a677, a678) {
                    let a681 = {
                        let a679 = v0;
                        let a680 = 8u32;
                        fixed_u32::bit_and(a679, a680)
                    };
                    let a682 = 0u32;
                    if equal(a681, a682) {
                        let a685 = {
                            let a683 = v0;
                            let a684 = 4u32;
                            fixed_u32::bit_and(a683, a684)
                        };
                        let a686 = 0u32;
                        if equal(a685, a686) {
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
                                    Bytes::lit(&[0xa0])
                                } else {
                                    Bytes::lit(&[0xa1])
                                }
                            } else {
                                let a697 = {
                                    let a695 = v0;
                                    let a696 = 1u32;
                                    fixed_u32::bit_and(a695, a696)
                                };
                                let a698 = 0u32;
                                if equal(a697, a698) {
                                    Bytes::lit(&[0xa2])
                                } else {
                                    Bytes::lit(&[0xa3])
                                }
                            }
                        } else {
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
                                    Bytes::lit(&[0xa4])
                                } else {
                                    Bytes::lit(&[0xa5])
                                }
                            } else {
                                let a709 = {
                                    let a707 = v0;
                                    let a708 = 1u32;
                                    fixed_u32::bit_and(a707, a708)
                                };
                                let a710 = 0u32;
                                if equal(a709, a710) {
                                    Bytes::lit(&[0xa6])
                                } else {
                                    Bytes::lit(&[0xa7])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0xa8])
                                } else {
                                    Bytes::lit(&[0xa9])
                                }
                            } else {
                                let a725 = {
                                    let a723 = v0;
                                    let a724 = 1u32;
                                    fixed_u32::bit_and(a723, a724)
                                };
                                let a726 = 0u32;
                                if equal(a725, a726) {
                                    Bytes::lit(&[0xaa])
                                } else {
                                    Bytes::lit(&[0xab])
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
                                    Bytes::lit(&[0xac])
                                } else {
                                    Bytes::lit(&[0xad])
                                }
                            } else {
                                let a737 = {
                                    let a735 = v0;
                                    let a736 = 1u32;
                                    fixed_u32::bit_and(a735, a736)
                                };
                                let a738 = 0u32;
                                if equal(a737, a738) {
                                    Bytes::lit(&[0xae])
                                } else {
                                    Bytes::lit(&[0xaf])
                                }
                            }
                        }
                    }
                } else {
                    let a741 = {
                        let a739 = v0;
                        let a740 = 8u32;
                        fixed_u32::bit_and(a739, a740)
                    };
                    let a742 = 0u32;
                    if equal(a741, a742) {
                        let a745 = {
                            let a743 = v0;
                            let a744 = 4u32;
                            fixed_u32::bit_and(a743, a744)
                        };
                        let a746 = 0u32;
                        if equal(a745, a746) {
                            let a749 = {
                                let a747 = v0;
                                let a748 = 2u32;
                                fixed_u32::bit_and(a747, a748)
                            };
                            let a750 = 0u32;
                            if equal(a749, a750) {
                                let a753 = {
                                    let a751 = v0;
                                    let a752 = 1u32;
                                    fixed_u32::bit_and(a751, a752)
                                };
                                let a754 = 0u32;
                                if equal(a753, a754) {
                                    Bytes::lit(&[0xb0])
                                } else {
                                    Bytes::lit(&[0xb1])
                                }
                            } else {
                                let a757 = {
                                    let a755 = v0;
                                    let a756 = 1u32;
                                    fixed_u32::bit_and(a755, a756)
                                };
                                let a758 = 0u32;
                                if equal(a757, a758) {
                                    Bytes::lit(&[0xb2])
                                } else {
                                    Bytes::lit(&[0xb3])
                                }
                            }
                        } else {
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
                                    Bytes::lit(&[0xb4])
                                } else {
                                    Bytes::lit(&[0xb5])
                                }
                            } else {
                                let a769 = {
                                    let a767 = v0;
                                    let a768 = 1u32;
                                    fixed_u32::bit_and(a767, a768)
                                };
                                let a770 = 0u32;
                                if equal(a769, a770) {
                                    Bytes::lit(&[0xb6])
                                } else {
                                    Bytes::lit(&[0xb7])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0xb8])
                                } else {
                                    Bytes::lit(&[0xb9])
                                }
                            } else {
                                let a785 = {
                                    let a783 = v0;
                                    let a784 = 1u32;
                                    fixed_u32::bit_and(a783, a784)
                                };
                                let a786 = 0u32;
                                if equal(a785, a786) {
                                    Bytes::lit(&[0xba])
                                } else {
                                    Bytes::lit(&[0xbb])
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
                                    Bytes::lit(&[0xbc])
                                } else {
                                    Bytes::lit(&[0xbd])
                                }
                            } else {
                                let a797 = {
                                    let a795 = v0;
                                    let a796 = 1u32;
                                    fixed_u32::bit_and(a795, a796)
                                };
                                let a798 = 0u32;
                                if equal(a797, a798) {
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
            let a801 = {
                let a799 = v0;
                let a800 = 32u32;
                fixed_u32::bit_and(a799, a800)
            };
            let a802 = 0u32;
            if equal(a801, a802) {
                let a805 = {
                    let a803 = v0;
                    let a804 = 16u32;
                    fixed_u32::bit_and(a803, a804)
                };
                let a806 = 0u32;
                if equal(a805, a806) {
                    let a809 = {
                        let a807 = v0;
                        let a808 = 8u32;
                        fixed_u32::bit_and(a807, a808)
                    };
                    let a810 = 0u32;
                    if equal(a809, a810) {
                        let a813 = {
                            let a811 = v0;
                            let a812 = 4u32;
                            fixed_u32::bit_and(a811, a812)
                        };
                        let a814 = 0u32;
                        if equal(a813, a814) {
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
                                    Bytes::lit(&[0xc0])
                                } else {
                                    Bytes::lit(&[0xc1])
                                }
                            } else {
                                let a825 = {
                                    let a823 = v0;
                                    let a824 = 1u32;
                                    fixed_u32::bit_and(a823, a824)
                                };
                                let a826 = 0u32;
                                if equal(a825, a826) {
                                    Bytes::lit(&[0xc2])
                                } else {
                                    Bytes::lit(&[0xc3])
                                }
                            }
                        } else {
                            let a829 = {
                                let a827 = v0;
                                let a828 = 2u32;
                                fixed_u32::bit_and(a827, a828)
                            };
                            let a830 = 0u32;
                            if equal(a829, a830) {
                                let a833 = {
                                    let a831 = v0;
                                    let a832 = 1u32;
                                    fixed_u32::bit_and(a831, a832)
                                };
                                let a834 = 0u32;
                                if equal(a833, a834) {
                                    Bytes::lit(&[0xc4])
                                } else {
                                    Bytes::lit(&[0xc5])
                                }
                            } else {
                                let a837 = {
                                    let a835 = v0;
                                    let a836 = 1u32;
                                    fixed_u32::bit_and(a835, a836)
                                };
                                let a838 = 0u32;
                                if equal(a837, a838) {
                                    Bytes::lit(&[0xc6])
                                } else {
                                    Bytes::lit(&[0xc7])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0xc8])
                                } else {
                                    Bytes::lit(&[0xc9])
                                }
                            } else {
                                let a853 = {
                                    let a851 = v0;
                                    let a852 = 1u32;
                                    fixed_u32::bit_and(a851, a852)
                                };
                                let a854 = 0u32;
                                if equal(a853, a854) {
                                    Bytes::lit(&[0xca])
                                } else {
                                    Bytes::lit(&[0xcb])
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
                                    Bytes::lit(&[0xcc])
                                } else {
                                    Bytes::lit(&[0xcd])
                                }
                            } else {
                                let a865 = {
                                    let a863 = v0;
                                    let a864 = 1u32;
                                    fixed_u32::bit_and(a863, a864)
                                };
                                let a866 = 0u32;
                                if equal(a865, a866) {
                                    Bytes::lit(&[0xce])
                                } else {
                                    Bytes::lit(&[0xcf])
                                }
                            }
                        }
                    }
                } else {
                    let a869 = {
                        let a867 = v0;
                        let a868 = 8u32;
                        fixed_u32::bit_and(a867, a868)
                    };
                    let a870 = 0u32;
                    if equal(a869, a870) {
                        let a873 = {
                            let a871 = v0;
                            let a872 = 4u32;
                            fixed_u32::bit_and(a871, a872)
                        };
                        let a874 = 0u32;
                        if equal(a873, a874) {
                            let a877 = {
                                let a875 = v0;
                                let a876 = 2u32;
                                fixed_u32::bit_and(a875, a876)
                            };
                            let a878 = 0u32;
                            if equal(a877, a878) {
                                let a881 = {
                                    let a879 = v0;
                                    let a880 = 1u32;
                                    fixed_u32::bit_and(a879, a880)
                                };
                                let a882 = 0u32;
                                if equal(a881, a882) {
                                    Bytes::lit(&[0xd0])
                                } else {
                                    Bytes::lit(&[0xd1])
                                }
                            } else {
                                let a885 = {
                                    let a883 = v0;
                                    let a884 = 1u32;
                                    fixed_u32::bit_and(a883, a884)
                                };
                                let a886 = 0u32;
                                if equal(a885, a886) {
                                    Bytes::lit(&[0xd2])
                                } else {
                                    Bytes::lit(&[0xd3])
                                }
                            }
                        } else {
                            let a889 = {
                                let a887 = v0;
                                let a888 = 2u32;
                                fixed_u32::bit_and(a887, a888)
                            };
                            let a890 = 0u32;
                            if equal(a889, a890) {
                                let a893 = {
                                    let a891 = v0;
                                    let a892 = 1u32;
                                    fixed_u32::bit_and(a891, a892)
                                };
                                let a894 = 0u32;
                                if equal(a893, a894) {
                                    Bytes::lit(&[0xd4])
                                } else {
                                    Bytes::lit(&[0xd5])
                                }
                            } else {
                                let a897 = {
                                    let a895 = v0;
                                    let a896 = 1u32;
                                    fixed_u32::bit_and(a895, a896)
                                };
                                let a898 = 0u32;
                                if equal(a897, a898) {
                                    Bytes::lit(&[0xd6])
                                } else {
                                    Bytes::lit(&[0xd7])
                                }
                            }
                        }
                    } else {
                        let a901 = {
                            let a899 = v0;
                            let a900 = 4u32;
                            fixed_u32::bit_and(a899, a900)
                        };
                        let a902 = 0u32;
                        if equal(a901, a902) {
                            let a905 = {
                                let a903 = v0;
                                let a904 = 2u32;
                                fixed_u32::bit_and(a903, a904)
                            };
                            let a906 = 0u32;
                            if equal(a905, a906) {
                                let a909 = {
                                    let a907 = v0;
                                    let a908 = 1u32;
                                    fixed_u32::bit_and(a907, a908)
                                };
                                let a910 = 0u32;
                                if equal(a909, a910) {
                                    Bytes::lit(&[0xd8])
                                } else {
                                    Bytes::lit(&[0xd9])
                                }
                            } else {
                                let a913 = {
                                    let a911 = v0;
                                    let a912 = 1u32;
                                    fixed_u32::bit_and(a911, a912)
                                };
                                let a914 = 0u32;
                                if equal(a913, a914) {
                                    Bytes::lit(&[0xda])
                                } else {
                                    Bytes::lit(&[0xdb])
                                }
                            }
                        } else {
                            let a917 = {
                                let a915 = v0;
                                let a916 = 2u32;
                                fixed_u32::bit_and(a915, a916)
                            };
                            let a918 = 0u32;
                            if equal(a917, a918) {
                                let a921 = {
                                    let a919 = v0;
                                    let a920 = 1u32;
                                    fixed_u32::bit_and(a919, a920)
                                };
                                let a922 = 0u32;
                                if equal(a921, a922) {
                                    Bytes::lit(&[0xdc])
                                } else {
                                    Bytes::lit(&[0xdd])
                                }
                            } else {
                                let a925 = {
                                    let a923 = v0;
                                    let a924 = 1u32;
                                    fixed_u32::bit_and(a923, a924)
                                };
                                let a926 = 0u32;
                                if equal(a925, a926) {
                                    Bytes::lit(&[0xde])
                                } else {
                                    Bytes::lit(&[0xdf])
                                }
                            }
                        }
                    }
                }
            } else {
                let a929 = {
                    let a927 = v0;
                    let a928 = 16u32;
                    fixed_u32::bit_and(a927, a928)
                };
                let a930 = 0u32;
                if equal(a929, a930) {
                    let a933 = {
                        let a931 = v0;
                        let a932 = 8u32;
                        fixed_u32::bit_and(a931, a932)
                    };
                    let a934 = 0u32;
                    if equal(a933, a934) {
                        let a937 = {
                            let a935 = v0;
                            let a936 = 4u32;
                            fixed_u32::bit_and(a935, a936)
                        };
                        let a938 = 0u32;
                        if equal(a937, a938) {
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
                                    Bytes::lit(&[0xe0])
                                } else {
                                    Bytes::lit(&[0xe1])
                                }
                            } else {
                                let a949 = {
                                    let a947 = v0;
                                    let a948 = 1u32;
                                    fixed_u32::bit_and(a947, a948)
                                };
                                let a950 = 0u32;
                                if equal(a949, a950) {
                                    Bytes::lit(&[0xe2])
                                } else {
                                    Bytes::lit(&[0xe3])
                                }
                            }
                        } else {
                            let a953 = {
                                let a951 = v0;
                                let a952 = 2u32;
                                fixed_u32::bit_and(a951, a952)
                            };
                            let a954 = 0u32;
                            if equal(a953, a954) {
                                let a957 = {
                                    let a955 = v0;
                                    let a956 = 1u32;
                                    fixed_u32::bit_and(a955, a956)
                                };
                                let a958 = 0u32;
                                if equal(a957, a958) {
                                    Bytes::lit(&[0xe4])
                                } else {
                                    Bytes::lit(&[0xe5])
                                }
                            } else {
                                let a961 = {
                                    let a959 = v0;
                                    let a960 = 1u32;
                                    fixed_u32::bit_and(a959, a960)
                                };
                                let a962 = 0u32;
                                if equal(a961, a962) {
                                    Bytes::lit(&[0xe6])
                                } else {
                                    Bytes::lit(&[0xe7])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0xe8])
                                } else {
                                    Bytes::lit(&[0xe9])
                                }
                            } else {
                                let a977 = {
                                    let a975 = v0;
                                    let a976 = 1u32;
                                    fixed_u32::bit_and(a975, a976)
                                };
                                let a978 = 0u32;
                                if equal(a977, a978) {
                                    Bytes::lit(&[0xea])
                                } else {
                                    Bytes::lit(&[0xeb])
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
                                    Bytes::lit(&[0xec])
                                } else {
                                    Bytes::lit(&[0xed])
                                }
                            } else {
                                let a989 = {
                                    let a987 = v0;
                                    let a988 = 1u32;
                                    fixed_u32::bit_and(a987, a988)
                                };
                                let a990 = 0u32;
                                if equal(a989, a990) {
                                    Bytes::lit(&[0xee])
                                } else {
                                    Bytes::lit(&[0xef])
                                }
                            }
                        }
                    }
                } else {
                    let a993 = {
                        let a991 = v0;
                        let a992 = 8u32;
                        fixed_u32::bit_and(a991, a992)
                    };
                    let a994 = 0u32;
                    if equal(a993, a994) {
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
                                    Bytes::lit(&[0xf0])
                                } else {
                                    Bytes::lit(&[0xf1])
                                }
                            } else {
                                let a1009 = {
                                    let a1007 = v0;
                                    let a1008 = 1u32;
                                    fixed_u32::bit_and(a1007, a1008)
                                };
                                let a1010 = 0u32;
                                if equal(a1009, a1010) {
                                    Bytes::lit(&[0xf2])
                                } else {
                                    Bytes::lit(&[0xf3])
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
                                    Bytes::lit(&[0xf4])
                                } else {
                                    Bytes::lit(&[0xf5])
                                }
                            } else {
                                let a1021 = {
                                    let a1019 = v0;
                                    let a1020 = 1u32;
                                    fixed_u32::bit_and(a1019, a1020)
                                };
                                let a1022 = 0u32;
                                if equal(a1021, a1022) {
                                    Bytes::lit(&[0xf6])
                                } else {
                                    Bytes::lit(&[0xf7])
                                }
                            }
                        }
                    } else {
                        let a1025 = {
                            let a1023 = v0;
                            let a1024 = 4u32;
                            fixed_u32::bit_and(a1023, a1024)
                        };
                        let a1026 = 0u32;
                        if equal(a1025, a1026) {
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
                                    Bytes::lit(&[0xf8])
                                } else {
                                    Bytes::lit(&[0xf9])
                                }
                            } else {
                                let a1037 = {
                                    let a1035 = v0;
                                    let a1036 = 1u32;
                                    fixed_u32::bit_and(a1035, a1036)
                                };
                                let a1038 = 0u32;
                                if equal(a1037, a1038) {
                                    Bytes::lit(&[0xfa])
                                } else {
                                    Bytes::lit(&[0xfb])
                                }
                            }
                        } else {
                            let a1041 = {
                                let a1039 = v0;
                                let a1040 = 2u32;
                                fixed_u32::bit_and(a1039, a1040)
                            };
                            let a1042 = 0u32;
                            if equal(a1041, a1042) {
                                let a1045 = {
                                    let a1043 = v0;
                                    let a1044 = 1u32;
                                    fixed_u32::bit_and(a1043, a1044)
                                };
                                let a1046 = 0u32;
                                if equal(a1045, a1046) {
                                    Bytes::lit(&[0xfc])
                                } else {
                                    Bytes::lit(&[0xfd])
                                }
                            } else {
                                let a1049 = {
                                    let a1047 = v0;
                                    let a1048 = 1u32;
                                    fixed_u32::bit_and(a1047, a1048)
                                };
                                let a1050 = 0u32;
                                if equal(a1049, a1050) {
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

pub fn f13(v0: u64, v1: u64) -> u32 {
    let (_, h1051) = {
        let (_, h1052) = f30(Fn4::F31, v1, (v0, (1u32, 0u32)));
        h1052
    };
    h1051
}

pub fn f14(v0: u32) -> Bytes {
    let a1063 = f12({
        let a1053 = v0;
        let a1054 = 255u32;
        fixed_u32::bit_and(a1053, a1054)
    });
    let a1064 = {
        let a1061 = f12({
            let a1055 = f32(v0, 8u32);
            let a1056 = 255u32;
            fixed_u32::bit_and(a1055, a1056)
        });
        let a1062 = {
            let a1059 = f12({
                let a1057 = f32(v0, 16u32);
                let a1058 = 255u32;
                fixed_u32::bit_and(a1057, a1058)
            });
            let a1060 = f12(f32(v0, 24u32));
            append_bytes(a1059, a1060)
        };
        append_bytes(a1061, a1062)
    };
    append_bytes(a1063, a1064)
}

pub fn f15(v0: u64) -> u32 {
    f13({
        let a1065 = v0;
        let a1066 = 4294967296u64;
        let a1067 = 0u64;
        nat_rem(a1065, a1066, a1067)
    }, 32u64)
}

pub fn f16(v0: u64) -> u32 {
    let a1068 = v0;
    let a1069 = 4294967296u64;
    if nat_lt(a1068, a1069) {
        0u32
    } else {
        f13({
            let a1070 = v0;
            let a1071 = 4294967296u64;
            let a1072 = 0u64;
            nat_quot(a1070, a1071, a1072)
        }, 32u64)
    }
}

pub fn f17(v0: Fn0, p1076: List<Bytes>, p1077: List<Bytes>) -> List<Bytes> {
    let m1075 = (p1076, p1077);
    {
        let mut m1075 = m1075;
        loop {
            let step = {
                let (v1, v2) = m1075;
                let m1073 = v2;
                match m1073.uncons() {
                    None => {
                        Err::<(List<Bytes>, List<Bytes>), List<Bytes>>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(List<Bytes>, List<Bytes>), List<Bytes>>(({
                            let c1074: Fn0 = v0.clone();
                            c1074.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m1075 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f18(v0: List<Bytes>, v1: Bytes) -> List<Bytes> {
    List::cons(v1, v0)
}

pub fn f19(v0: Fn1, p1082: u64, p1083: List<Bytes>) -> (List<Bytes>, bool) {
    let m1081 = (p1082, p1083);
    {
        let mut m1081 = m1081;
        loop {
            let step = {
                let (v1, v2) = m1081;
                let m1078 = v1;
                if m1078 == 0 {
                    Err::<(u64, List<Bytes>), (List<Bytes>, bool)>((v2, false))
                } else {
                    let v3 = m1078 - 1;
                    let m1080 = {
                        let c1079: Fn1 = v0.clone();
                        c1079.apply(v2.clone())
                    };
                    match m1080 {
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
                Ok(next) => { m1081 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f20(v0: List<Bytes>) -> Option<List<Bytes>> {
    let a1085 = {
        let a1084 = v0.clone();
        length_list(a1084)
    };
    let a1086 = 1u64;
    if nat_le(a1085, a1086) {
        None::<List<Bytes>>
    } else {
        Some(f33(v0))
    }
}

pub fn f21(v0: Adt0) -> R<Result<Adt9, Adt1>> {
    let m1087 = v0;
    match m1087 {
        Adt0::C0 => {
            Ok(Ok::<Adt9, Adt1>(f34(f9(), 0u32)))
        }
        Adt0::C1(v1) => {
            let a1089 = {
                let a1088 = v1.clone();
                length_bytes(a1088)
            };
            let a1090 = 32u64;
            if nat_eq(a1089, a1090) {
                Ok(Ok::<Adt9, Adt1>(f34(f35(v1)?, f36())))
            } else {
                Ok(Err::<Adt9, Adt1>(Adt1::C0({
                    let a1091 = v1;
                    length_bytes(a1091)
                })))
            }
        }
        Adt0::C2(v2) => {
            Ok(Ok::<Adt9, Adt1>(f37(v2)?))
        }
    }
}

pub fn f22(v0: Adt9, v1: u64) -> R<Adt8> {
    let v2: Adt5 = match v0.clone() {
        Adt9::C0(h1092, _, _, _, _, _, _) => {
            h1092
        }
    };
    let v3: u32 = match v0.clone() {
        Adt9::C0(_, h1093, _, _, _, _, _) => {
            h1093
        }
    };
    let v4: Bytes = match v0.clone() {
        Adt9::C0(_, _, _, h1094, _, _, _) => {
            h1094
        }
    };
    let v5: (Adt10, List<Adt3>) = f38(v2.clone(), v3, v4.clone(), 0u64, {
        let a1095 = v4.clone();
        length_bytes(a1095)
    }, match v0.clone() {
        Adt9::C0(_, _, h1096, _, _, _, _) => {
            h1096
        }
    }, match v0.clone() {
        Adt9::C0(_, _, _, _, _, h1097, _) => {
            h1097
        }
    })?;
    let v6: (Adt10, (List<Adt3>, u64)) = f39(Fn5::F40(v3, v2.clone()), ({
        let (h1098, _) = v5;
        h1098
    }, ({
        let (_, h1099) = v5;
        h1099
    }, {
        let a1101 = match v0.clone() {
            Adt9::C0(_, _, _, _, h1100, _, _) => {
                h1100
            }
        };
        length_list(a1101)
    })), match v0.clone() {
        Adt9::C0(_, _, _, _, h1102, _, _) => {
            h1102
        }
    })?;
    let v7: (u64, List<Adt3>) = f41(Fn6::F42(v6.clone()), {
        let a1105 = {
            let a1103 = v1;
            let a1104 = 63u64;
            nat_add(a1103, a1104)?
        };
        let a1106 = 64u64;
        let a1107 = 0u64;
        nat_quot(a1105, a1106, a1107)
    }, (0u64, {
        let (h1108, _) = {
            let (_, h1109) = v6;
            h1109
        };
        h1108
    }))?;
    Ok(Adt8::C0(f43(v3), {
        let a1114 = {
            let a1111 = match v0.clone() {
                Adt9::C0(_, _, h1110, _, _, _, _) => {
                    h1110
                }
            };
            let a1112 = 1024u64;
            nat_mul(a1111, a1112)?
        };
        let a1115 = {
            let a1113 = v4;
            length_bytes(a1113)
        };
        nat_add(a1114, a1115)?
    }, v1, v2, v3, match v0 {
        Adt9::C0(_, _, _, _, _, _, h1116) => {
            h1116
        }
    }, f44({
        let (_, h1117) = v7;
        h1117
    })))
}

pub fn f23(v0: Fn2, p1121: Adt9, p1122: List<Bytes>) -> R<Adt9> {
    let m1120 = (p1121, p1122);
    {
        let mut m1120 = m1120;
        loop {
            let step = {
                let (v1, v2) = m1120;
                let m1118 = v2;
                match m1118.uncons() {
                    None => {
                        Err::<(Adt9, List<Bytes>), R<Adt9>>(Ok(v1))
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt9, List<Bytes>), R<Adt9>>(({
                            let c1119: Fn2 = v0.clone();
                            c1119.apply(v1, v3)?
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m1120 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f24(v0: Adt9, v1: Bytes) -> R<Adt9> {
    f45(v0, v1)
}

pub fn f25(v0: Adt2, v1: Adt8, v2: Bytes, v3: Bytes) -> R<(Adt2, Bytes)> {
    let v4: u64 = {
        let a1124 = match v1.clone() {
            Adt8::C0(_, h1123, _, _, _, _, _) => {
                h1123
            }
        };
        let a1125 = 0u64;
        if nat_eq(a1124, a1125) {
            1u64
        } else {
            let a1129 = {
                let a1127 = match v1.clone() {
                    Adt8::C0(_, h1126, _, _, _, _, _) => {
                        h1126
                    }
                };
                let a1128 = 1023u64;
                nat_add(a1127, a1128)?
            };
            let a1130 = 1024u64;
            let a1131 = 0u64;
            nat_quot(a1129, a1130, a1131)
        }
    };
    let v6: Bytes = {
        let m1133 = match v0.clone() {
            Adt2::C0(_, _, _, _, _, _, h1132, _, _, _, _) => {
                h1132
            }
        };
        match m1133 {
            None => {
                f46(0u64, v0.clone(), Bytes::lit(&[0x00]))
            }
            Some(v5) => {
                let a1143 = Bytes::lit(&[0x03]);
                let a1144 = {
                    let a1141 = f3(0u64);
                    let a1142 = {
                        let a1139 = f3(v4);
                        let a1140 = {
                            let a1137 = v5;
                            let a1138 = {
                                let a1135 = match v0.clone() {
                                    Adt2::C0(_, _, _, _, _, _, _, h1134, _, _, _) => {
                                        h1134
                                    }
                                };
                                let a1136 = Bytes::lit(&[0x00]);
                                append_bytes(a1135, a1136)
                            };
                            append_bytes(a1137, a1138)
                        };
                        append_bytes(a1139, a1140)
                    };
                    append_bytes(a1141, a1142)
                };
                append_bytes(a1143, a1144)
            }
        }
    };
    let v7: (Adt2, Bytes) = f47(v0.clone(), v6, Bytes::lit(&[]))?;
    let v8: (Adt2, Bytes) = f47({
        let (h1145, _) = v7;
        h1145
    }, {
        let a1150 = Bytes::lit(&[0x05]);
        let a1151 = {
            let a1148 = f3(match v0.clone() {
                Adt2::C0(_, _, _, _, _, _, _, _, _, _, h1146) => {
                    h1146
                }
            });
            let a1149 = f4(match v0.clone() {
                Adt2::C0(_, _, _, _, _, _, _, _, _, h1147, _) => {
                    h1147
                }
            });
            append_bytes(a1148, a1149)
        };
        append_bytes(a1150, a1151)
    }, Bytes::lit(&[]))?;
    let v9: u64 = match v1.clone() {
        Adt8::C0(h1152, _, _, _, _, _, _) => {
            h1152
        }
    };
    f47({
        let (h1153, _) = v8;
        h1153
    }, f5(List::cons(Bytes::lit(&[0x06, 0x01]), List::cons(f2(v9), List::cons(f3(match v1.clone() {
        Adt8::C0(_, h1154, _, _, _, _, _) => {
            h1154
        }
    }), List::cons(f3(match v1.clone() {
        Adt8::C0(_, _, h1155, _, _, _, _) => {
            h1155
        }
    }), List::cons(v2, List::cons({
        let (_, h1156) = v7;
        h1156
    }, List::cons({
        let (_, h1157) = v8;
        h1157
    }, List::cons({
        let a1158 = v9;
        let a1159 = 2u64;
        if nat_eq(a1158, a1159) {
            v3
        } else {
            Bytes::lit(&[])
        }
    }, List::cons(f3({
        let a1161 = match v0.clone() {
            Adt2::C0(_, _, _, _, _, _, _, _, h1160, _, _) => {
                h1160
            }
        };
        length_list(a1161)
    }), List::cons(f4(match v0 {
        Adt2::C0(_, _, _, _, _, _, _, _, h1162, _, _) => {
            h1162
        }
    }), List::cons(f48(v1), List::<Bytes>::nil())))))))))))), Bytes::lit(&[]))
}

pub fn f26(v0: Fn3, p1166: Adt2, p1167: List<Adt3>) -> R<Adt2> {
    let m1165 = (p1166, p1167);
    {
        let mut m1165 = m1165;
        loop {
            let step = {
                let (v1, v2) = m1165;
                let m1163 = v2;
                match m1163.uncons() {
                    None => {
                        Err::<(Adt2, List<Adt3>), R<Adt2>>(Ok(v1))
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt2, List<Adt3>), R<Adt2>>(({
                            let c1164: Fn3 = v0.clone();
                            c1164.apply(v1, v3)?
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m1165 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f27(v0: Adt2, v1: Adt3) -> R<Adt2> {
    f49(v0, v1)
}

pub fn f28(v0: Bytes, v1: u64) -> R<Bytes> {
    Ok(f48(f22(f45(f34(f9(), 0u32), v0)?, v1)?))
}

pub fn f29(v0: Bytes) -> R<Bytes> {
    let v1: Adt11 = f50(v0)?;
    let a1188 = f51(match v1.clone() {
        Adt11::C0(h1168, _, _, _, _, _, _, _) => {
            h1168
        }
    });
    let a1189 = {
        let a1186 = f51(match v1.clone() {
            Adt11::C0(_, h1169, _, _, _, _, _, _) => {
                h1169
            }
        });
        let a1187 = {
            let a1184 = f51(match v1.clone() {
                Adt11::C0(_, _, h1170, _, _, _, _, _) => {
                    h1170
                }
            });
            let a1185 = {
                let a1182 = f51(match v1.clone() {
                    Adt11::C0(_, _, _, h1171, _, _, _, _) => {
                        h1171
                    }
                });
                let a1183 = {
                    let a1180 = f51(match v1.clone() {
                        Adt11::C0(_, _, _, _, h1172, _, _, _) => {
                            h1172
                        }
                    });
                    let a1181 = {
                        let a1178 = f51(match v1.clone() {
                            Adt11::C0(_, _, _, _, _, h1173, _, _) => {
                                h1173
                            }
                        });
                        let a1179 = {
                            let a1176 = f51(match v1.clone() {
                                Adt11::C0(_, _, _, _, _, _, h1174, _) => {
                                    h1174
                                }
                            });
                            let a1177 = f51(match v1 {
                                Adt11::C0(_, _, _, _, _, _, _, h1175) => {
                                    h1175
                                }
                            });
                            append_bytes(a1176, a1177)
                        };
                        append_bytes(a1178, a1179)
                    };
                    append_bytes(a1180, a1181)
                };
                append_bytes(a1182, a1183)
            };
            append_bytes(a1184, a1185)
        };
        append_bytes(a1186, a1187)
    };
    Ok(append_bytes(a1188, a1189))
}

pub fn f30(v0: Fn4, p1193: u64, p1194: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let m1192 = (p1193, p1194);
    {
        let mut m1192 = m1192;
        loop {
            let step = {
                let (v1, v2) = m1192;
                let m1190 = v1;
                if m1190 == 0 {
                    Err::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>(v2)
                } else {
                    let v3 = m1190 - 1;
                    Ok::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>((v3, {
                        let c1191: Fn4 = v0.clone();
                        c1191.apply(v2)
                    }))
                }
            };
            match step {
                Ok(next) => { m1192 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f31(v0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let v1: u64 = {
        let (h1195, _) = v0;
        h1195
    };
    let v2: u32 = {
        let (h1196, _) = {
            let (_, h1197) = v0;
            h1197
        };
        h1196
    };
    let v3: u32 = {
        let (_, h1198) = {
            let (_, h1199) = v0;
            h1199
        };
        h1198
    };
    ({
        let a1200 = v1;
        let a1201 = 2u64;
        let a1202 = 0u64;
        nat_quot(a1200, a1201, a1202)
    }, (f52(v2, 1u32), {
        let a1206 = {
            let a1203 = v1;
            let a1204 = 2u64;
            let a1205 = 0u64;
            nat_rem(a1203, a1204, a1205)
        };
        let a1207 = 1u64;
        if nat_eq(a1206, a1207) {
            let a1208 = v3;
            let a1209 = v2;
            fixed_u32::bit_or(a1208, a1209)
        } else {
            v3
        }
    }))
}

pub fn f32(v0: u32, v1: u32) -> u32 {
    let m1212 = {
        let a1210 = v0;
        let a1211 = v1;
        fixed_u32::shift_right(a1210, a1211)
    };
    match m1212 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f33(v0: List<Bytes>) -> List<Bytes> {
    let v1: (Option<Bytes>, List<Bytes>) = f53(Fn7::F54, (None::<Bytes>, List::<Bytes>::nil()), v0);
    let v3: List<Bytes> = {
        let m1214 = {
            let (h1213, _) = v1;
            h1213
        };
        match m1214 {
            None => {
                let (_, h1216) = v1;
                h1216
            }
            Some(v2) => {
                List::cons(v2, {
                    let (_, h1215) = v1;
                    h1215
                })
            }
        }
    };
    f17(Fn0::F55, List::<Bytes>::nil(), v3)
}

pub fn f34(v0: Adt5, v1: u32) -> Adt9 {
    Adt9::C0(v0, v1, 0u64, Bytes::lit(&[]), List::<Adt7>::nil(), List::<Adt3>::nil(), List::<Adt3>::nil())
}

pub fn f35(v0: Bytes) -> R<Adt5> {
    Ok(f56(f57(v0.clone(), {
        let a1217 = v0;
        length_bytes(a1217)
    }, 0u64)?))
}

pub fn f36() -> u32 {
    16u32
}

pub fn f37(v0: Bytes) -> R<Adt9> {
    let v1: Adt8 = f58(v0)?;
    Ok(Adt9::C0(f56(f59(v1.clone(), 0u64)), f60(), 0u64, Bytes::lit(&[]), List::<Adt7>::nil(), List::<Adt3>::nil(), match v1 {
        Adt8::C0(_, _, _, _, _, _, h1218) => {
            h1218
        }
    }))
}

pub fn f38(v0: Adt5, v1: u32, v2: Bytes, v3: u64, v4: u64, v5: u64, v6: List<Adt3>) -> R<(Adt10, List<Adt3>)> {
    let v7: u64 = {
        let a1219 = v3;
        let a1220 = v4;
        nat_add(a1219, a1220)?
    };
    let v8: u64 = f61(v4)?;
    let v9: (u64, (Adt5, List<Adt3>)) = f62(Fn8::F63(v1, v3, v5, v0.clone(), v7, v2.clone()), {
        let a1221 = v8;
        let a1222 = 64u64;
        let a1223 = 0u64;
        nat_quot(a1221, a1222, a1223)
    }, (0u64, (v0.clone(), v6)))?;
    Ok((f64(v0, v1, v2, v7, v3, v5, v8, {
        let (h1224, _) = {
            let (_, h1225) = v9.clone();
            h1225
        };
        h1224
    }, {
        let a1226 = v4;
        let a1227 = v8;
        nat_sub(a1226, a1227)
    }, {
        let a1228 = f65();
        let a1229 = f66(v8);
        fixed_u32::bit_or(a1228, a1229)
    })?, {
        let (_, h1230) = {
            let (_, h1231) = v9;
            h1231
        };
        h1230
    }))
}

pub fn f39(v0: Fn5, p1235: (Adt10, (List<Adt3>, u64)), p1236: List<Adt7>) -> R<(Adt10, (List<Adt3>, u64))> {
    let m1234 = (p1235, p1236);
    {
        let mut m1234 = m1234;
        loop {
            let step = {
                let (v1, v2) = m1234;
                let m1232 = v2;
                match m1232.uncons() {
                    None => {
                        Err::<((Adt10, (List<Adt3>, u64)), List<Adt7>), R<(Adt10, (List<Adt3>, u64))>>(Ok(v1))
                    }
                    Some((v3, v4)) => {
                        Ok::<((Adt10, (List<Adt3>, u64)), List<Adt7>), R<(Adt10, (List<Adt3>, u64))>>(({
                            let c1233: Fn5 = v0.clone();
                            c1233.apply(v1, v3)?
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m1234 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f40(v0: u32, v1: Adt5, v2: (Adt10, (List<Adt3>, u64)), v3: Adt7) -> R<(Adt10, (List<Adt3>, u64))> {
    let v4: Adt4 = f67({
        let (h1237, _) = v2.clone();
        h1237
    })?;
    Ok((f68(v3.clone(), Adt7::C0(f56(match v4.clone() {
        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1238) => {
            h1238
        }
    }), {
        let m1241 = {
            let (h1240, _) = v2.clone();
            h1240
        };
        match m1241 {
            Adt10::C0(_, h1239, _, _, _, _, _, _, _, _) => {
                h1239
            }
        }
    }, {
        let m1244 = {
            let (h1243, _) = v2.clone();
            h1243
        };
        match m1244 {
            Adt10::C0(_, _, h1242, _, _, _, _, _, _, _) => {
                h1242
            }
        }
    }), v1, v0)?, (List::cons(Adt3::C2(v3, {
        let (_, h1245) = {
            let (_, h1246) = v2.clone();
            h1246
        };
        h1245
    }), List::cons(Adt3::C0(v4), {
        let (h1247, _) = {
            let (_, h1248) = v2.clone();
            h1248
        };
        h1247
    })), {
        let a1251 = {
            let (_, h1249) = {
                let (_, h1250) = v2;
                h1250
            };
            h1249
        };
        let a1252 = 1u64;
        nat_sub(a1251, a1252)
    })))
}

pub fn f41(v0: Fn6, p1256: u64, p1257: (u64, List<Adt3>)) -> R<(u64, List<Adt3>)> {
    let m1255 = (p1256, p1257);
    {
        let mut m1255 = m1255;
        loop {
            let step = {
                let (v1, v2) = m1255;
                let m1253 = v1;
                if m1253 == 0 {
                    Err::<(u64, (u64, List<Adt3>)), R<(u64, List<Adt3>)>>(Ok(v2))
                } else {
                    let v3 = m1253 - 1;
                    Ok::<(u64, (u64, List<Adt3>)), R<(u64, List<Adt3>)>>((v3, {
                        let c1254: Fn6 = v0.clone();
                        c1254.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m1255 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f42(v0: (Adt10, (List<Adt3>, u64)), v1: (u64, List<Adt3>)) -> R<(u64, List<Adt3>)> {
    Ok(({
        let a1259 = {
            let (h1258, _) = v1.clone();
            h1258
        };
        let a1260 = 1u64;
        nat_add(a1259, a1260)?
    }, List::cons(Adt3::C0(f67(f69({
        let (h1261, _) = v0;
        h1261
    }, {
        let (h1262, _) = v1;
        h1262
    }))?), {
        let (_, h1263) = v1;
        h1263
    })))
}

pub fn f43(v0: u32) -> u64 {
    let a1264 = v0;
    let a1265 = f36();
    if equal(a1264, a1265) {
        1u64
    } else {
        let a1266 = v0;
        let a1267 = f60();
        if equal(a1266, a1267) {
            2u64
        } else {
            let a1268 = v0;
            let a1269 = f10();
            if equal(a1268, a1269) {
                3u64
            } else {
                0u64
            }
        }
    }
}

pub fn f44(v0: List<Adt3>) -> List<Adt3> {
    f70(Fn9::F71, List::<Adt3>::nil(), v0)
}

pub fn f45(v0: Adt9, v1: Bytes) -> R<Adt9> {
    let v2: Bytes = {
        let a1271 = match v0.clone() {
            Adt9::C0(_, _, _, h1270, _, _, _) => {
                h1270
            }
        };
        let a1272 = v1;
        append_bytes(a1271, a1272)
    };
    let v3: u64 = {
        let a1273 = v2.clone();
        length_bytes(a1273)
    };
    let v4: u64 = {
        let a1274 = v3;
        let a1275 = 0u64;
        if nat_eq(a1274, a1275) {
            0u64
        } else {
            let a1278 = {
                let a1276 = v3;
                let a1277 = 1u64;
                nat_sub(a1276, a1277)
            };
            let a1279 = 1024u64;
            let a1280 = 0u64;
            nat_quot(a1278, a1279, a1280)
        }
    };
    let v5: u32 = match v0.clone() {
        Adt9::C0(_, h1281, _, _, _, _, _) => {
            h1281
        }
    };
    let v6: Adt5 = match v0.clone() {
        Adt9::C0(h1282, _, _, _, _, _, _) => {
            h1282
        }
    };
    let v7: (u64, (List<Adt7>, List<Adt3>)) = f72(Fn10::F73(v5, v2.clone(), v6.clone(), v0.clone()), v4, (0u64, (match v0.clone() {
        Adt9::C0(_, _, _, _, h1283, _, _) => {
            h1283
        }
    }, match v0.clone() {
        Adt9::C0(_, _, _, _, _, h1284, _) => {
            h1284
        }
    })))?;
    let v8: u64 = {
        let a1285 = v4;
        let a1286 = 1024u64;
        nat_mul(a1285, a1286)?
    };
    Ok(Adt9::C0(v6, v5, {
        let a1288 = match v0.clone() {
            Adt9::C0(_, _, h1287, _, _, _, _) => {
                h1287
            }
        };
        let a1289 = v4;
        nat_add(a1288, a1289)?
    }, {
        let m1295 = {
            let a1292 = v2;
            let a1293 = v8;
            let a1294 = {
                let a1290 = v3;
                let a1291 = v8;
                nat_sub(a1290, a1291)
            };
            slice_bytes(a1292, a1293, a1294)
        };
        match m1295 {
            None => {
                Bytes::lit(&[])
            }
            Some(v9) => {
                v9
            }
        }
    }, {
        let (h1296, _) = {
            let (_, h1297) = v7.clone();
            h1297
        };
        h1296
    }, {
        let (_, h1298) = {
            let (_, h1299) = v7;
            h1299
        };
        h1298
    }, match v0 {
        Adt9::C0(_, _, _, _, _, _, h1300) => {
            h1300
        }
    }))
}

pub fn f46(v0: u64, v1: Adt2, v2: Bytes) -> Bytes {
    let a1309 = Bytes::lit(&[0x02]);
    let a1310 = {
        let a1307 = f3(v0);
        let a1308 = {
            let a1305 = f2(match v1.clone() {
                Adt2::C0(_, _, _, _, _, h1301, _, _, _, _, _) => {
                    h1301
                }
            });
            let a1306 = {
                let a1303 = f4(match v1 {
                    Adt2::C0(_, _, _, _, h1302, _, _, _, _, _, _) => {
                        h1302
                    }
                });
                let a1304 = v2;
                append_bytes(a1303, a1304)
            };
            append_bytes(a1305, a1306)
        };
        append_bytes(a1307, a1308)
    };
    append_bytes(a1309, a1310)
}

pub fn f47(v0: Adt2, v1: Bytes, v2: Bytes) -> R<(Adt2, Bytes)> {
    let v3: Bytes = f11(v1.clone())?;
    Ok((Adt2::C0(match v0.clone() {
        Adt2::C0(h1311, _, _, _, _, _, _, _, _, _, _) => {
            h1311
        }
    }, List::cons({
        let a1312 = f74(v1, v3.clone());
        let a1313 = v2;
        append_bytes(a1312, a1313)
    }, match v0.clone() {
        Adt2::C0(_, h1314, _, _, _, _, _, _, _, _, _) => {
            h1314
        }
    }), {
        let a1316 = match v0.clone() {
            Adt2::C0(_, _, h1315, _, _, _, _, _, _, _, _) => {
                h1315
            }
        };
        let a1317 = 1u64;
        nat_add(a1316, a1317)?
    }, match v0.clone() {
        Adt2::C0(_, _, _, h1318, _, _, _, _, _, _, _) => {
            h1318
        }
    }, match v0.clone() {
        Adt2::C0(_, _, _, _, h1319, _, _, _, _, _, _) => {
            h1319
        }
    }, match v0.clone() {
        Adt2::C0(_, _, _, _, _, h1320, _, _, _, _, _) => {
            h1320
        }
    }, match v0.clone() {
        Adt2::C0(_, _, _, _, _, _, h1321, _, _, _, _) => {
            h1321
        }
    }, match v0.clone() {
        Adt2::C0(_, _, _, _, _, _, _, h1322, _, _, _) => {
            h1322
        }
    }, match v0.clone() {
        Adt2::C0(_, _, _, _, _, _, _, _, h1323, _, _) => {
            h1323
        }
    }, match v0.clone() {
        Adt2::C0(_, _, _, _, _, _, _, _, _, h1324, _) => {
            h1324
        }
    }, match v0 {
        Adt2::C0(_, _, _, _, _, _, _, _, _, _, h1325) => {
            h1325
        }
    }), f75(v3)))
}

pub fn f48(v0: Adt8) -> Bytes {
    let v1: List<Bytes> = f76(Fn11::F77, List::<Bytes>::nil(), match v0.clone() {
        Adt8::C0(_, _, _, _, _, _, h1326) => {
            h1326
        }
    });
    let v2: Bytes = f5(f17(Fn0::F78, List::<Bytes>::nil(), v1));
    let m1331 = {
        let a1328 = v2.clone();
        let a1329 = 0u64;
        let a1330 = match v0 {
            Adt8::C0(_, _, h1327, _, _, _, _) => {
                h1327
            }
        };
        slice_bytes(a1328, a1329, a1330)
    };
    match m1331 {
        None => {
            v2
        }
        Some(v3) => {
            v3
        }
    }
}

pub fn f49(v0: Adt2, v1: Adt3) -> R<Adt2> {
    let m1332 = v1;
    match m1332 {
        Adt3::C0(v2) => {
            let v3: Bytes = f79(v2.clone());
            let v4: (Adt2, Bytes) = f47(v0.clone(), v3, {
                let a1334 = match v0.clone() {
                    Adt2::C0(h1333, _, _, _, _, _, _, _, _, _, _) => {
                        h1333
                    }
                };
                let a1335 = 0u64;
                if nat_eq(a1334, a1335) {
                    Bytes::lit(&[])
                } else {
                    f80(match v0 {
                        Adt2::C0(h1336, _, _, _, _, _, _, _, _, _, _) => {
                            h1336
                        }
                    }, v2.clone())?
                }
            })?;
            let v5: Adt2 = {
                let (h1337, _) = v4;
                h1337
            };
            let v6: Bytes = {
                let (_, h1338) = v4;
                h1338
            };
            let v7: List<Bytes> = List::cons({
                let a1339 = Bytes::lit(&[0x00]);
                let a1340 = v6.clone();
                append_bytes(a1339, a1340)
            }, match v5.clone() {
                Adt2::C0(_, _, _, _, _, _, _, _, _, h1341, _) => {
                    h1341
                }
            });
            let a1343 = match v2.clone() {
                Adt4::C0(h1342, _, _, _, _, _, _, _, _, _, _, _) => {
                    h1342
                }
            };
            let a1344 = 0u64;
            if nat_eq(a1343, a1344) {
                let a1348 = {
                    let a1346 = match v2.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1345, _) => {
                            h1345
                        }
                    };
                    let a1347 = f65();
                    fixed_u32::bit_and(a1346, a1347)
                };
                let a1349 = f65();
                if equal(a1348, a1349) {
                    let v8: (Adt2, Bytes) = f47(v5.clone(), f46(match v2.clone() {
                        Adt4::C0(_, h1350, _, _, _, _, _, _, _, _, _, _) => {
                            h1350
                        }
                    }, Adt2::C0(match v5.clone() {
                        Adt2::C0(h1351, _, _, _, _, _, _, _, _, _, _) => {
                            h1351
                        }
                    }, match v5.clone() {
                        Adt2::C0(_, h1352, _, _, _, _, _, _, _, _, _) => {
                            h1352
                        }
                    }, match v5.clone() {
                        Adt2::C0(_, _, h1353, _, _, _, _, _, _, _, _) => {
                            h1353
                        }
                    }, match v5.clone() {
                        Adt2::C0(_, _, _, h1354, _, _, _, _, _, _, _) => {
                            h1354
                        }
                    }, List::cons(v6, match v5.clone() {
                        Adt2::C0(_, _, _, _, h1355, _, _, _, _, _, _) => {
                            h1355
                        }
                    }), {
                        let a1357 = match v5.clone() {
                            Adt2::C0(_, _, _, _, _, h1356, _, _, _, _, _) => {
                                h1356
                            }
                        };
                        let a1358 = 1u64;
                        nat_add(a1357, a1358)?
                    }, match v5.clone() {
                        Adt2::C0(_, _, _, _, _, _, h1359, _, _, _, _) => {
                            h1359
                        }
                    }, match v5.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, h1360, _, _, _) => {
                            h1360
                        }
                    }, match v5.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, _, h1361, _, _) => {
                            h1361
                        }
                    }, match v5.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, _, _, h1362, _) => {
                            h1362
                        }
                    }, match v5.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, _, _, _, h1363) => {
                            h1363
                        }
                    }), {
                        let a1365 = Bytes::lit(&[0x01]);
                        let a1366 = f81(f56(match v2 {
                            Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1364) => {
                                h1364
                            }
                        }));
                        append_bytes(a1365, a1366)
                    }), Bytes::lit(&[]))?;
                    let v9: Adt2 = {
                        let (h1367, _) = v8;
                        h1367
                    };
                    Ok(Adt2::C0(match v9.clone() {
                        Adt2::C0(h1368, _, _, _, _, _, _, _, _, _, _) => {
                            h1368
                        }
                    }, match v9.clone() {
                        Adt2::C0(_, h1369, _, _, _, _, _, _, _, _, _) => {
                            h1369
                        }
                    }, match v9.clone() {
                        Adt2::C0(_, _, h1370, _, _, _, _, _, _, _, _) => {
                            h1370
                        }
                    }, match v9.clone() {
                        Adt2::C0(_, _, _, h1371, _, _, _, _, _, _, _) => {
                            h1371
                        }
                    }, List::<Bytes>::nil(), 0u64, match v9.clone() {
                        Adt2::C0(_, _, _, _, _, _, h1372, _, _, _, _) => {
                            h1372
                        }
                    }, {
                        let (_, h1373) = v8;
                        h1373
                    }, match v9 {
                        Adt2::C0(_, _, _, _, _, _, _, _, h1374, _, _) => {
                            h1374
                        }
                    }, v7, {
                        let a1376 = match v5 {
                            Adt2::C0(_, _, _, _, _, _, _, _, _, _, h1375) => {
                                h1375
                            }
                        };
                        let a1377 = 1u64;
                        nat_add(a1376, a1377)?
                    }))
                } else {
                    Ok(Adt2::C0(match v5.clone() {
                        Adt2::C0(h1378, _, _, _, _, _, _, _, _, _, _) => {
                            h1378
                        }
                    }, match v5.clone() {
                        Adt2::C0(_, h1379, _, _, _, _, _, _, _, _, _) => {
                            h1379
                        }
                    }, match v5.clone() {
                        Adt2::C0(_, _, h1380, _, _, _, _, _, _, _, _) => {
                            h1380
                        }
                    }, match v5.clone() {
                        Adt2::C0(_, _, _, h1381, _, _, _, _, _, _, _) => {
                            h1381
                        }
                    }, List::cons(v6, match v5.clone() {
                        Adt2::C0(_, _, _, _, h1382, _, _, _, _, _, _) => {
                            h1382
                        }
                    }), {
                        let a1384 = match v5.clone() {
                            Adt2::C0(_, _, _, _, _, h1383, _, _, _, _, _) => {
                                h1383
                            }
                        };
                        let a1385 = 1u64;
                        nat_add(a1384, a1385)?
                    }, match v5.clone() {
                        Adt2::C0(_, _, _, _, _, _, h1386, _, _, _, _) => {
                            h1386
                        }
                    }, match v5.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, h1387, _, _, _) => {
                            h1387
                        }
                    }, match v5.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, _, h1388, _, _) => {
                            h1388
                        }
                    }, v7, {
                        let a1390 = match v5 {
                            Adt2::C0(_, _, _, _, _, _, _, _, _, _, h1389) => {
                                h1389
                            }
                        };
                        let a1391 = 1u64;
                        nat_add(a1390, a1391)?
                    }))
                }
            } else {
                let a1393 = match v2.clone() {
                    Adt4::C0(h1392, _, _, _, _, _, _, _, _, _, _, _) => {
                        h1392
                    }
                };
                let a1394 = 1u64;
                if nat_eq(a1393, a1394) {
                    let m1396 = match v5.clone() {
                        Adt2::C0(_, _, _, _, _, _, h1395, _, _, _, _) => {
                            h1395
                        }
                    };
                    match m1396 {
                        None => {
                            Ok(Adt2::C0(match v5.clone() {
                                Adt2::C0(h1427, _, _, _, _, _, _, _, _, _, _) => {
                                    h1427
                                }
                            }, match v5.clone() {
                                Adt2::C0(_, h1428, _, _, _, _, _, _, _, _, _) => {
                                    h1428
                                }
                            }, match v5.clone() {
                                Adt2::C0(_, _, h1429, _, _, _, _, _, _, _, _) => {
                                    h1429
                                }
                            }, match v5.clone() {
                                Adt2::C0(_, _, _, h1430, _, _, _, _, _, _, _) => {
                                    h1430
                                }
                            }, match v5.clone() {
                                Adt2::C0(_, _, _, _, h1431, _, _, _, _, _, _) => {
                                    h1431
                                }
                            }, match v5.clone() {
                                Adt2::C0(_, _, _, _, _, h1432, _, _, _, _, _) => {
                                    h1432
                                }
                            }, match v5.clone() {
                                Adt2::C0(_, _, _, _, _, _, h1433, _, _, _, _) => {
                                    h1433
                                }
                            }, match v5.clone() {
                                Adt2::C0(_, _, _, _, _, _, _, h1434, _, _, _) => {
                                    h1434
                                }
                            }, match v5.clone() {
                                Adt2::C0(_, _, _, _, _, _, _, _, h1435, _, _) => {
                                    h1435
                                }
                            }, v7, {
                                let a1437 = match v5 {
                                    Adt2::C0(_, _, _, _, _, _, _, _, _, _, h1436) => {
                                        h1436
                                    }
                                };
                                let a1438 = 1u64;
                                nat_add(a1437, a1438)?
                            }))
                        }
                        Some(v10) => {
                            let v11: (Adt2, Bytes) = f47(v5.clone(), {
                                let a1413 = Bytes::lit(&[0x03]);
                                let a1414 = {
                                    let a1411 = f3(match v2.clone() {
                                        Adt4::C0(_, h1397, _, _, _, _, _, _, _, _, _, _) => {
                                            h1397
                                        }
                                    });
                                    let a1412 = {
                                        let a1409 = f3(match v2.clone() {
                                            Adt4::C0(_, _, h1398, _, _, _, _, _, _, _, _, _) => {
                                                h1398
                                            }
                                        });
                                        let a1410 = {
                                            let a1407 = v10;
                                            let a1408 = {
                                                let a1405 = match v5.clone() {
                                                    Adt2::C0(_, _, _, _, _, _, _, h1399, _, _, _) => {
                                                        h1399
                                                    }
                                                };
                                                let a1406 = {
                                                    let a1403 = Bytes::lit(&[0x01]);
                                                    let a1404 = {
                                                        let a1401 = v6;
                                                        let a1402 = f81(f56(match v2 {
                                                            Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1400) => {
                                                                h1400
                                                            }
                                                        }));
                                                        append_bytes(a1401, a1402)
                                                    };
                                                    append_bytes(a1403, a1404)
                                                };
                                                append_bytes(a1405, a1406)
                                            };
                                            append_bytes(a1407, a1408)
                                        };
                                        append_bytes(a1409, a1410)
                                    };
                                    append_bytes(a1411, a1412)
                                };
                                append_bytes(a1413, a1414)
                            }, Bytes::lit(&[]))?;
                            let v12: Adt2 = {
                                let (h1415, _) = v11;
                                h1415
                            };
                            Ok(Adt2::C0(match v12.clone() {
                                Adt2::C0(h1416, _, _, _, _, _, _, _, _, _, _) => {
                                    h1416
                                }
                            }, match v12.clone() {
                                Adt2::C0(_, h1417, _, _, _, _, _, _, _, _, _) => {
                                    h1417
                                }
                            }, match v12.clone() {
                                Adt2::C0(_, _, h1418, _, _, _, _, _, _, _, _) => {
                                    h1418
                                }
                            }, match v12.clone() {
                                Adt2::C0(_, _, _, h1419, _, _, _, _, _, _, _) => {
                                    h1419
                                }
                            }, match v12.clone() {
                                Adt2::C0(_, _, _, _, h1420, _, _, _, _, _, _) => {
                                    h1420
                                }
                            }, match v12.clone() {
                                Adt2::C0(_, _, _, _, _, h1421, _, _, _, _, _) => {
                                    h1421
                                }
                            }, None::<Bytes>, {
                                let (_, h1422) = v11;
                                h1422
                            }, match v12 {
                                Adt2::C0(_, _, _, _, _, _, _, _, h1423, _, _) => {
                                    h1423
                                }
                            }, v7, {
                                let a1425 = match v5 {
                                    Adt2::C0(_, _, _, _, _, _, _, _, _, _, h1424) => {
                                        h1424
                                    }
                                };
                                let a1426 = 1u64;
                                nat_add(a1425, a1426)?
                            }))
                        }
                    }
                } else {
                    let v13: (Adt2, Bytes) = f47(v5.clone(), {
                        let a1445 = Bytes::lit(&[0x04]);
                        let a1446 = {
                            let a1443 = f3(match v2.clone() {
                                Adt4::C0(_, _, _, h1439, _, _, _, _, _, _, _, _) => {
                                    h1439
                                }
                            });
                            let a1444 = {
                                let a1441 = v6;
                                let a1442 = f82(match v2 {
                                    Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1440) => {
                                        h1440
                                    }
                                });
                                append_bytes(a1441, a1442)
                            };
                            append_bytes(a1443, a1444)
                        };
                        append_bytes(a1445, a1446)
                    }, Bytes::lit(&[]))?;
                    let v14: Adt2 = {
                        let (h1447, _) = v13;
                        h1447
                    };
                    Ok(Adt2::C0(match v14.clone() {
                        Adt2::C0(h1448, _, _, _, _, _, _, _, _, _, _) => {
                            h1448
                        }
                    }, match v14.clone() {
                        Adt2::C0(_, h1449, _, _, _, _, _, _, _, _, _) => {
                            h1449
                        }
                    }, match v14.clone() {
                        Adt2::C0(_, _, h1450, _, _, _, _, _, _, _, _) => {
                            h1450
                        }
                    }, match v14.clone() {
                        Adt2::C0(_, _, _, h1451, _, _, _, _, _, _, _) => {
                            h1451
                        }
                    }, match v14.clone() {
                        Adt2::C0(_, _, _, _, h1452, _, _, _, _, _, _) => {
                            h1452
                        }
                    }, match v14.clone() {
                        Adt2::C0(_, _, _, _, _, h1453, _, _, _, _, _) => {
                            h1453
                        }
                    }, match v14.clone() {
                        Adt2::C0(_, _, _, _, _, _, h1454, _, _, _, _) => {
                            h1454
                        }
                    }, match v14.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, h1455, _, _, _) => {
                            h1455
                        }
                    }, List::cons({
                        let (_, h1456) = v13;
                        h1456
                    }, match v14 {
                        Adt2::C0(_, _, _, _, _, _, _, _, h1457, _, _) => {
                            h1457
                        }
                    }), v7, {
                        let a1459 = match v5 {
                            Adt2::C0(_, _, _, _, _, _, _, _, _, _, h1458) => {
                                h1458
                            }
                        };
                        let a1460 = 1u64;
                        nat_add(a1459, a1460)?
                    }))
                }
            }
        }
        Adt3::C1(v15, v16) => {
            Ok(Adt2::C0(match v0.clone() {
                Adt2::C0(h1461, _, _, _, _, _, _, _, _, _, _) => {
                    h1461
                }
            }, match v0.clone() {
                Adt2::C0(_, h1462, _, _, _, _, _, _, _, _, _) => {
                    h1462
                }
            }, match v0.clone() {
                Adt2::C0(_, _, h1463, _, _, _, _, _, _, _, _) => {
                    h1463
                }
            }, List::cons(match v0.clone() {
                Adt2::C0(_, _, _, _, _, _, _, h1464, _, _, _) => {
                    h1464
                }
            }, match v0.clone() {
                Adt2::C0(_, _, _, h1465, _, _, _, _, _, _, _) => {
                    h1465
                }
            }), match v0.clone() {
                Adt2::C0(_, _, _, _, h1466, _, _, _, _, _, _) => {
                    h1466
                }
            }, match v0.clone() {
                Adt2::C0(_, _, _, _, _, h1467, _, _, _, _, _) => {
                    h1467
                }
            }, match v0.clone() {
                Adt2::C0(_, _, _, _, _, _, h1468, _, _, _, _) => {
                    h1468
                }
            }, match v0.clone() {
                Adt2::C0(_, _, _, _, _, _, _, h1469, _, _, _) => {
                    h1469
                }
            }, match v0.clone() {
                Adt2::C0(_, _, _, _, _, _, _, _, h1470, _, _) => {
                    h1470
                }
            }, List::cons({
                let a1480 = Bytes::lit(&[0x01]);
                let a1481 = {
                    let a1478 = f2(v16);
                    let a1479 = {
                        let a1476 = f3(match v15.clone() {
                            Adt7::C0(_, h1471, _) => {
                                h1471
                            }
                        });
                        let a1477 = {
                            let a1474 = f3(match v15.clone() {
                                Adt7::C0(_, _, h1472) => {
                                    h1472
                                }
                            });
                            let a1475 = f81(match v15 {
                                Adt7::C0(h1473, _, _) => {
                                    h1473
                                }
                            });
                            append_bytes(a1474, a1475)
                        };
                        append_bytes(a1476, a1477)
                    };
                    append_bytes(a1478, a1479)
                };
                append_bytes(a1480, a1481)
            }, match v0.clone() {
                Adt2::C0(_, _, _, _, _, _, _, _, _, h1482, _) => {
                    h1482
                }
            }), {
                let a1484 = match v0 {
                    Adt2::C0(_, _, _, _, _, _, _, _, _, _, h1483) => {
                        h1483
                    }
                };
                let a1485 = 1u64;
                nat_add(a1484, a1485)?
            }))
        }
        Adt3::C2(v17, v18) => {
            let m1487 = match v0.clone() {
                Adt2::C0(_, _, _, h1486, _, _, _, _, _, _, _) => {
                    h1486
                }
            };
            match m1487.uncons() {
                None => {
                    Ok(Adt2::C0(match v0.clone() {
                        Adt2::C0(h1510, _, _, _, _, _, _, _, _, _, _) => {
                            h1510
                        }
                    }, match v0.clone() {
                        Adt2::C0(_, h1511, _, _, _, _, _, _, _, _, _) => {
                            h1511
                        }
                    }, match v0.clone() {
                        Adt2::C0(_, _, h1512, _, _, _, _, _, _, _, _) => {
                            h1512
                        }
                    }, match v0.clone() {
                        Adt2::C0(_, _, _, h1513, _, _, _, _, _, _, _) => {
                            h1513
                        }
                    }, match v0.clone() {
                        Adt2::C0(_, _, _, _, h1514, _, _, _, _, _, _) => {
                            h1514
                        }
                    }, match v0.clone() {
                        Adt2::C0(_, _, _, _, _, h1515, _, _, _, _, _) => {
                            h1515
                        }
                    }, match v0.clone() {
                        Adt2::C0(_, _, _, _, _, _, h1516, _, _, _, _) => {
                            h1516
                        }
                    }, match v0.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, h1517, _, _, _) => {
                            h1517
                        }
                    }, match v0.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, _, h1518, _, _) => {
                            h1518
                        }
                    }, List::cons({
                        let a1528 = Bytes::lit(&[0x02]);
                        let a1529 = {
                            let a1526 = f2(v18);
                            let a1527 = {
                                let a1524 = f3(match v17.clone() {
                                    Adt7::C0(_, h1519, _) => {
                                        h1519
                                    }
                                });
                                let a1525 = {
                                    let a1522 = f3(match v17.clone() {
                                        Adt7::C0(_, _, h1520) => {
                                            h1520
                                        }
                                    });
                                    let a1523 = f81(match v17 {
                                        Adt7::C0(h1521, _, _) => {
                                            h1521
                                        }
                                    });
                                    append_bytes(a1522, a1523)
                                };
                                append_bytes(a1524, a1525)
                            };
                            append_bytes(a1526, a1527)
                        };
                        append_bytes(a1528, a1529)
                    }, match v0.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, _, _, h1530, _) => {
                            h1530
                        }
                    }), {
                        let a1532 = match v0 {
                            Adt2::C0(_, _, _, _, _, _, _, _, _, _, h1531) => {
                                h1531
                            }
                        };
                        let a1533 = 1u64;
                        nat_add(a1532, a1533)?
                    }))
                }
                Some((v19, v20)) => {
                    Ok(Adt2::C0(match v0.clone() {
                        Adt2::C0(h1488, _, _, _, _, _, _, _, _, _, _) => {
                            h1488
                        }
                    }, match v0.clone() {
                        Adt2::C0(_, h1489, _, _, _, _, _, _, _, _, _) => {
                            h1489
                        }
                    }, match v0.clone() {
                        Adt2::C0(_, _, h1490, _, _, _, _, _, _, _, _) => {
                            h1490
                        }
                    }, v20, match v0.clone() {
                        Adt2::C0(_, _, _, _, h1491, _, _, _, _, _, _) => {
                            h1491
                        }
                    }, match v0.clone() {
                        Adt2::C0(_, _, _, _, _, h1492, _, _, _, _, _) => {
                            h1492
                        }
                    }, Some(v19), match v0.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, h1493, _, _, _) => {
                            h1493
                        }
                    }, match v0.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, _, h1494, _, _) => {
                            h1494
                        }
                    }, List::cons({
                        let a1504 = Bytes::lit(&[0x02]);
                        let a1505 = {
                            let a1502 = f2(v18);
                            let a1503 = {
                                let a1500 = f3(match v17.clone() {
                                    Adt7::C0(_, h1495, _) => {
                                        h1495
                                    }
                                });
                                let a1501 = {
                                    let a1498 = f3(match v17.clone() {
                                        Adt7::C0(_, _, h1496) => {
                                            h1496
                                        }
                                    });
                                    let a1499 = f81(match v17 {
                                        Adt7::C0(h1497, _, _) => {
                                            h1497
                                        }
                                    });
                                    append_bytes(a1498, a1499)
                                };
                                append_bytes(a1500, a1501)
                            };
                            append_bytes(a1502, a1503)
                        };
                        append_bytes(a1504, a1505)
                    }, match v0.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, _, _, h1506, _) => {
                            h1506
                        }
                    }), {
                        let a1508 = match v0 {
                            Adt2::C0(_, _, _, _, _, _, _, _, _, _, h1507) => {
                                h1507
                            }
                        };
                        let a1509 = 1u64;
                        nat_add(a1508, a1509)?
                    }))
                }
            }
        }
    }
}

pub fn f50(v0: Bytes) -> R<Adt11> {
    let v1: u64 = {
        let a1534 = v0.clone();
        length_bytes(a1534)
    };
    let v2: u64 = {
        let a1540 = {
            let a1537 = {
                let a1535 = v1;
                let a1536 = 72u64;
                nat_add(a1535, a1536)?
            };
            let a1538 = 64u64;
            let a1539 = 0u64;
            nat_quot(a1537, a1538, a1539)
        };
        let a1541 = 64u64;
        nat_mul(a1540, a1541)?
    };
    let (_, h1542) = f83(Fn12::F84(v0, v1, v2), {
        let a1543 = v2;
        let a1544 = 64u64;
        let a1545 = 0u64;
        nat_quot(a1543, a1544, a1545)
    }, (0u64, f85()))?;
    Ok(h1542)
}

pub fn f51(v0: u32) -> Bytes {
    let a1558 = f12({
        let a1546 = f32(v0, 24u32);
        let a1547 = 255u32;
        fixed_u32::bit_and(a1546, a1547)
    });
    let a1559 = {
        let a1556 = f12({
            let a1548 = f32(v0, 16u32);
            let a1549 = 255u32;
            fixed_u32::bit_and(a1548, a1549)
        });
        let a1557 = {
            let a1554 = f12({
                let a1550 = f32(v0, 8u32);
                let a1551 = 255u32;
                fixed_u32::bit_and(a1550, a1551)
            });
            let a1555 = f12({
                let a1552 = f32(v0, 0u32);
                let a1553 = 255u32;
                fixed_u32::bit_and(a1552, a1553)
            });
            append_bytes(a1554, a1555)
        };
        append_bytes(a1556, a1557)
    };
    append_bytes(a1558, a1559)
}

pub fn f52(v0: u32, v1: u32) -> u32 {
    let m1562 = {
        let a1560 = v0;
        let a1561 = v1;
        fixed_u32::shift_left(a1560, a1561)
    };
    match m1562 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f53(v0: Fn7, p1566: (Option<Bytes>, List<Bytes>), p1567: List<Bytes>) -> (Option<Bytes>, List<Bytes>) {
    let m1565 = (p1566, p1567);
    {
        let mut m1565 = m1565;
        loop {
            let step = {
                let (v1, v2) = m1565;
                let m1563 = v2;
                match m1563.uncons() {
                    None => {
                        Err::<((Option<Bytes>, List<Bytes>), List<Bytes>), (Option<Bytes>, List<Bytes>)>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<((Option<Bytes>, List<Bytes>), List<Bytes>), (Option<Bytes>, List<Bytes>)>(({
                            let c1564: Fn7 = v0.clone();
                            c1564.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m1565 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f54(v0: (Option<Bytes>, List<Bytes>), v1: Bytes) -> (Option<Bytes>, List<Bytes>) {
    let m1569 = {
        let (h1568, _) = v0;
        h1568
    };
    match m1569 {
        None => {
            (Some(v1), {
                let (_, h1573) = v0;
                h1573
            })
        }
        Some(v2) => {
            (None::<Bytes>, List::cons({
                let a1570 = v2;
                let a1571 = v1;
                append_bytes(a1570, a1571)
            }, {
                let (_, h1572) = v0;
                h1572
            }))
        }
    }
}

pub fn f55(v0: List<Bytes>, v1: Bytes) -> List<Bytes> {
    List::cons(v1, v0)
}

pub fn f56(v0: Adt6) -> Adt5 {
    Adt5::C0(match v0.clone() {
        Adt6::C0(h1574, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1574
        }
    }, match v0.clone() {
        Adt6::C0(_, h1575, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1575
        }
    }, match v0.clone() {
        Adt6::C0(_, _, h1576, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1576
        }
    }, match v0.clone() {
        Adt6::C0(_, _, _, h1577, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1577
        }
    }, match v0.clone() {
        Adt6::C0(_, _, _, _, h1578, _, _, _, _, _, _, _, _, _, _, _) => {
            h1578
        }
    }, match v0.clone() {
        Adt6::C0(_, _, _, _, _, h1579, _, _, _, _, _, _, _, _, _, _) => {
            h1579
        }
    }, match v0.clone() {
        Adt6::C0(_, _, _, _, _, _, h1580, _, _, _, _, _, _, _, _, _) => {
            h1580
        }
    }, match v0 {
        Adt6::C0(_, _, _, _, _, _, _, h1581, _, _, _, _, _, _, _, _) => {
            h1581
        }
    })
}

pub fn f57(v0: Bytes, v1: u64, v2: u64) -> R<Adt6> {
    Ok(Adt6::C0(f86(v0.clone(), v1, {
        let a1582 = v2;
        let a1583 = 0u64;
        nat_add(a1582, a1583)?
    })?, f86(v0.clone(), v1, {
        let a1584 = v2;
        let a1585 = 4u64;
        nat_add(a1584, a1585)?
    })?, f86(v0.clone(), v1, {
        let a1586 = v2;
        let a1587 = 8u64;
        nat_add(a1586, a1587)?
    })?, f86(v0.clone(), v1, {
        let a1588 = v2;
        let a1589 = 12u64;
        nat_add(a1588, a1589)?
    })?, f86(v0.clone(), v1, {
        let a1590 = v2;
        let a1591 = 16u64;
        nat_add(a1590, a1591)?
    })?, f86(v0.clone(), v1, {
        let a1592 = v2;
        let a1593 = 20u64;
        nat_add(a1592, a1593)?
    })?, f86(v0.clone(), v1, {
        let a1594 = v2;
        let a1595 = 24u64;
        nat_add(a1594, a1595)?
    })?, f86(v0.clone(), v1, {
        let a1596 = v2;
        let a1597 = 28u64;
        nat_add(a1596, a1597)?
    })?, f86(v0.clone(), v1, {
        let a1598 = v2;
        let a1599 = 32u64;
        nat_add(a1598, a1599)?
    })?, f86(v0.clone(), v1, {
        let a1600 = v2;
        let a1601 = 36u64;
        nat_add(a1600, a1601)?
    })?, f86(v0.clone(), v1, {
        let a1602 = v2;
        let a1603 = 40u64;
        nat_add(a1602, a1603)?
    })?, f86(v0.clone(), v1, {
        let a1604 = v2;
        let a1605 = 44u64;
        nat_add(a1604, a1605)?
    })?, f86(v0.clone(), v1, {
        let a1606 = v2;
        let a1607 = 48u64;
        nat_add(a1606, a1607)?
    })?, f86(v0.clone(), v1, {
        let a1608 = v2;
        let a1609 = 52u64;
        nat_add(a1608, a1609)?
    })?, f86(v0.clone(), v1, {
        let a1610 = v2;
        let a1611 = 56u64;
        nat_add(a1610, a1611)?
    })?, f86(v0, v1, {
        let a1612 = v2;
        let a1613 = 60u64;
        nat_add(a1612, a1613)?
    })?))
}

pub fn f58(v0: Bytes) -> R<Adt8> {
    f22(f45(f34(f9(), f10()), v0)?, 32u64)
}

pub fn f59(v0: Adt8, v1: u64) -> Adt6 {
    f87(Fn13::F88(v1), Adt6::C0(0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32), match v0 {
        Adt8::C0(_, _, _, _, _, _, h1614) => {
            h1614
        }
    })
}

pub fn f60() -> u32 {
    64u32
}

pub fn f61(v0: u64) -> R<u64> {
    let a1615 = v0;
    let a1616 = 0u64;
    if nat_eq(a1615, a1616) {
        Ok(0u64)
    } else {
        let a1622 = {
            let a1619 = {
                let a1617 = v0;
                let a1618 = 1u64;
                nat_sub(a1617, a1618)
            };
            let a1620 = 64u64;
            let a1621 = 0u64;
            nat_quot(a1619, a1620, a1621)
        };
        let a1623 = 64u64;
        nat_mul(a1622, a1623)
    }
}

pub fn f62(v0: Fn8, p1627: u64, p1628: (u64, (Adt5, List<Adt3>))) -> R<(u64, (Adt5, List<Adt3>))> {
    let m1626 = (p1627, p1628);
    {
        let mut m1626 = m1626;
        loop {
            let step = {
                let (v1, v2) = m1626;
                let m1624 = v1;
                if m1624 == 0 {
                    Err::<(u64, (u64, (Adt5, List<Adt3>))), R<(u64, (Adt5, List<Adt3>))>>(Ok(v2))
                } else {
                    let v3 = m1624 - 1;
                    Ok::<(u64, (u64, (Adt5, List<Adt3>))), R<(u64, (Adt5, List<Adt3>))>>((v3, {
                        let c1625: Fn8 = v0.clone();
                        c1625.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m1626 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f63(v0: u32, v1: u64, v2: u64, v3: Adt5, v4: u64, v5: Bytes, v6: (u64, (Adt5, List<Adt3>))) -> R<(u64, (Adt5, List<Adt3>))> {
    let v7: u64 = {
        let (h1629, _) = v6.clone();
        h1629
    };
    let v8: Adt4 = f67(f64(v3, v0, v5, v4, v1, v2, v7, {
        let (h1630, _) = {
            let (_, h1631) = v6.clone();
            h1631
        };
        h1630
    }, 64u64, f66(v7))?)?;
    Ok(({
        let a1632 = v7;
        let a1633 = 64u64;
        nat_add(a1632, a1633)?
    }, (f56(match v8.clone() {
        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1634) => {
            h1634
        }
    }), List::cons(Adt3::C0(v8), {
        let (_, h1635) = {
            let (_, h1636) = v6;
            h1636
        };
        h1635
    }))))
}

pub fn f64(_: Adt5, v1: u32, v2: Bytes, v3: u64, v4: u64, v5: u64, v6: u64, v7: Adt5, v8: u64, v9: u32) -> R<Adt10> {
    Ok(Adt10::C0(0u64, v5, 1u64, {
        let a1637 = v6;
        let a1638 = 64u64;
        let a1639 = 0u64;
        nat_quot(a1637, a1638, a1639)
    }, {
        let a1642 = {
            let a1640 = v5;
            let a1641 = 1024u64;
            nat_mul(a1640, a1641)?
        };
        let a1643 = v6;
        nat_add(a1642, a1643)?
    }, v7, f57(v2, v3, {
        let a1644 = v4;
        let a1645 = v6;
        nat_add(a1644, a1645)?
    })?, v5, f13(v8, 7u64), {
        let a1646 = v1;
        let a1647 = v9;
        fixed_u32::bit_or(a1646, a1647)
    }))
}

pub fn f65() -> u32 {
    2u32
}

pub fn f66(v0: u64) -> u32 {
    let a1648 = v0;
    let a1649 = 0u64;
    if nat_eq(a1648, a1649) {
        f89()
    } else {
        0u32
    }
}

pub fn f67(v0: Adt10) -> R<Adt4> {
    let v1: u32 = f15(match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, _, h1650, _, _) => {
            h1650
        }
    });
    let v2: u32 = f16(match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, _, h1651, _, _) => {
            h1651
        }
    });
    Ok(Adt4::C0(match v0.clone() {
        Adt10::C0(h1652, _, _, _, _, _, _, _, _, _) => {
            h1652
        }
    }, match v0.clone() {
        Adt10::C0(_, h1653, _, _, _, _, _, _, _, _) => {
            h1653
        }
    }, match v0.clone() {
        Adt10::C0(_, _, h1654, _, _, _, _, _, _, _) => {
            h1654
        }
    }, match v0.clone() {
        Adt10::C0(_, _, _, h1655, _, _, _, _, _, _) => {
            h1655
        }
    }, match v0.clone() {
        Adt10::C0(_, _, _, _, h1656, _, _, _, _, _) => {
            h1656
        }
    }, match v0.clone() {
        Adt10::C0(_, _, _, _, _, h1657, _, _, _, _) => {
            h1657
        }
    }, match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, h1658, _, _, _) => {
            h1658
        }
    }, v1, v2, match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, _, _, h1659, _) => {
            h1659
        }
    }, match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, _, _, _, h1660) => {
            h1660
        }
    }, f90(match v0.clone() {
        Adt10::C0(_, _, _, _, _, h1661, _, _, _, _) => {
            h1661
        }
    }, match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, h1662, _, _, _) => {
            h1662
        }
    }, v1, v2, match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, _, _, h1663, _) => {
            h1663
        }
    }, match v0 {
        Adt10::C0(_, _, _, _, _, _, _, _, _, h1664) => {
            h1664
        }
    })?))
}

pub fn f68(v0: Adt7, v1: Adt7, v2: Adt5, v3: u32) -> R<Adt10> {
    Ok(Adt10::C0(1u64, match v0.clone() {
        Adt7::C0(_, h1665, _) => {
            h1665
        }
    }, {
        let a1668 = match v0.clone() {
            Adt7::C0(_, _, h1666) => {
                h1666
            }
        };
        let a1669 = match v1.clone() {
            Adt7::C0(_, _, h1667) => {
                h1667
            }
        };
        nat_add(a1668, a1669)?
    }, 0u64, 0u64, v2, Adt6::C0({
        let m1672 = match v0.clone() {
            Adt7::C0(h1671, _, _) => {
                h1671
            }
        };
        match m1672 {
            Adt5::C0(h1670, _, _, _, _, _, _, _) => {
                h1670
            }
        }
    }, {
        let m1675 = match v0.clone() {
            Adt7::C0(h1674, _, _) => {
                h1674
            }
        };
        match m1675 {
            Adt5::C0(_, h1673, _, _, _, _, _, _) => {
                h1673
            }
        }
    }, {
        let m1678 = match v0.clone() {
            Adt7::C0(h1677, _, _) => {
                h1677
            }
        };
        match m1678 {
            Adt5::C0(_, _, h1676, _, _, _, _, _) => {
                h1676
            }
        }
    }, {
        let m1681 = match v0.clone() {
            Adt7::C0(h1680, _, _) => {
                h1680
            }
        };
        match m1681 {
            Adt5::C0(_, _, _, h1679, _, _, _, _) => {
                h1679
            }
        }
    }, {
        let m1684 = match v0.clone() {
            Adt7::C0(h1683, _, _) => {
                h1683
            }
        };
        match m1684 {
            Adt5::C0(_, _, _, _, h1682, _, _, _) => {
                h1682
            }
        }
    }, {
        let m1687 = match v0.clone() {
            Adt7::C0(h1686, _, _) => {
                h1686
            }
        };
        match m1687 {
            Adt5::C0(_, _, _, _, _, h1685, _, _) => {
                h1685
            }
        }
    }, {
        let m1690 = match v0.clone() {
            Adt7::C0(h1689, _, _) => {
                h1689
            }
        };
        match m1690 {
            Adt5::C0(_, _, _, _, _, _, h1688, _) => {
                h1688
            }
        }
    }, {
        let m1693 = match v0 {
            Adt7::C0(h1692, _, _) => {
                h1692
            }
        };
        match m1693 {
            Adt5::C0(_, _, _, _, _, _, _, h1691) => {
                h1691
            }
        }
    }, {
        let m1696 = match v1.clone() {
            Adt7::C0(h1695, _, _) => {
                h1695
            }
        };
        match m1696 {
            Adt5::C0(h1694, _, _, _, _, _, _, _) => {
                h1694
            }
        }
    }, {
        let m1699 = match v1.clone() {
            Adt7::C0(h1698, _, _) => {
                h1698
            }
        };
        match m1699 {
            Adt5::C0(_, h1697, _, _, _, _, _, _) => {
                h1697
            }
        }
    }, {
        let m1702 = match v1.clone() {
            Adt7::C0(h1701, _, _) => {
                h1701
            }
        };
        match m1702 {
            Adt5::C0(_, _, h1700, _, _, _, _, _) => {
                h1700
            }
        }
    }, {
        let m1705 = match v1.clone() {
            Adt7::C0(h1704, _, _) => {
                h1704
            }
        };
        match m1705 {
            Adt5::C0(_, _, _, h1703, _, _, _, _) => {
                h1703
            }
        }
    }, {
        let m1708 = match v1.clone() {
            Adt7::C0(h1707, _, _) => {
                h1707
            }
        };
        match m1708 {
            Adt5::C0(_, _, _, _, h1706, _, _, _) => {
                h1706
            }
        }
    }, {
        let m1711 = match v1.clone() {
            Adt7::C0(h1710, _, _) => {
                h1710
            }
        };
        match m1711 {
            Adt5::C0(_, _, _, _, _, h1709, _, _) => {
                h1709
            }
        }
    }, {
        let m1714 = match v1.clone() {
            Adt7::C0(h1713, _, _) => {
                h1713
            }
        };
        match m1714 {
            Adt5::C0(_, _, _, _, _, _, h1712, _) => {
                h1712
            }
        }
    }, {
        let m1717 = match v1 {
            Adt7::C0(h1716, _, _) => {
                h1716
            }
        };
        match m1717 {
            Adt5::C0(_, _, _, _, _, _, _, h1715) => {
                h1715
            }
        }
    }), 0u64, 64u32, {
        let a1718 = v3;
        let a1719 = f91();
        fixed_u32::bit_or(a1718, a1719)
    }))
}

pub fn f69(v0: Adt10, v1: u64) -> Adt10 {
    Adt10::C0(2u64, match v0.clone() {
        Adt10::C0(_, h1720, _, _, _, _, _, _, _, _) => {
            h1720
        }
    }, match v0.clone() {
        Adt10::C0(_, _, h1721, _, _, _, _, _, _, _) => {
            h1721
        }
    }, v1, match v0.clone() {
        Adt10::C0(_, _, _, _, h1722, _, _, _, _, _) => {
            h1722
        }
    }, match v0.clone() {
        Adt10::C0(_, _, _, _, _, h1723, _, _, _, _) => {
            h1723
        }
    }, match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, h1724, _, _, _) => {
            h1724
        }
    }, v1, match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, _, _, h1725, _) => {
            h1725
        }
    }, {
        let a1727 = match v0 {
            Adt10::C0(_, _, _, _, _, _, _, _, _, h1726) => {
                h1726
            }
        };
        let a1728 = f92();
        fixed_u32::bit_or(a1727, a1728)
    })
}

pub fn f70(v0: Fn9, p1732: List<Adt3>, p1733: List<Adt3>) -> List<Adt3> {
    let m1731 = (p1732, p1733);
    {
        let mut m1731 = m1731;
        loop {
            let step = {
                let (v1, v2) = m1731;
                let m1729 = v2;
                match m1729.uncons() {
                    None => {
                        Err::<(List<Adt3>, List<Adt3>), List<Adt3>>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(List<Adt3>, List<Adt3>), List<Adt3>>(({
                            let c1730: Fn9 = v0.clone();
                            c1730.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m1731 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f71(v0: List<Adt3>, v1: Adt3) -> List<Adt3> {
    List::cons(v1, v0)
}

pub fn f72(v0: Fn10, p1737: u64, p1738: (u64, (List<Adt7>, List<Adt3>))) -> R<(u64, (List<Adt7>, List<Adt3>))> {
    let m1736 = (p1737, p1738);
    {
        let mut m1736 = m1736;
        loop {
            let step = {
                let (v1, v2) = m1736;
                let m1734 = v1;
                if m1734 == 0 {
                    Err::<(u64, (u64, (List<Adt7>, List<Adt3>))), R<(u64, (List<Adt7>, List<Adt3>))>>(Ok(v2))
                } else {
                    let v3 = m1734 - 1;
                    Ok::<(u64, (u64, (List<Adt7>, List<Adt3>))), R<(u64, (List<Adt7>, List<Adt3>))>>((v3, {
                        let c1735: Fn10 = v0.clone();
                        c1735.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m1736 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f73(v0: u32, v1: Bytes, v2: Adt5, v3: Adt9, v4: (u64, (List<Adt7>, List<Adt3>))) -> R<(u64, (List<Adt7>, List<Adt3>))> {
    let v5: u64 = {
        let a1741 = match v3 {
            Adt9::C0(_, _, h1739, _, _, _, _) => {
                h1739
            }
        };
        let a1742 = {
            let (h1740, _) = v4.clone();
            h1740
        };
        nat_add(a1741, a1742)?
    };
    let v6: (Adt10, List<Adt3>) = f38(v2.clone(), v0, v1, {
        let a1744 = {
            let (h1743, _) = v4.clone();
            h1743
        };
        let a1745 = 1024u64;
        nat_mul(a1744, a1745)?
    }, 1024u64, v5, {
        let (_, h1746) = {
            let (_, h1747) = v4.clone();
            h1747
        };
        h1746
    })?;
    let v7: Adt4 = f67({
        let (h1748, _) = v6;
        h1748
    })?;
    Ok(({
        let a1750 = {
            let (h1749, _) = v4;
            h1749
        };
        let a1751 = 1u64;
        nat_add(a1750, a1751)?
    }, f93(v2, v0, Adt12::C0(Adt7::C0(f56(match v7.clone() {
        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1752) => {
            h1752
        }
    }), v5, 1u64), {
        let a1753 = v5;
        let a1754 = 1u64;
        nat_add(a1753, a1754)?
    }, {
        let (h1755, _) = {
            let (_, h1756) = v4;
            h1756
        };
        h1755
    }, List::cons(Adt3::C0(v7), {
        let (_, h1757) = v6;
        h1757
    })))?))
}

pub fn f74(v0: Bytes, v1: Bytes) -> Bytes {
    let a1761 = f94({
        let a1758 = v0.clone();
        length_bytes(a1758)
    });
    let a1762 = {
        let a1759 = v0;
        let a1760 = v1;
        append_bytes(a1759, a1760)
    };
    append_bytes(a1761, a1762)
}

pub fn f75(v0: Bytes) -> Bytes {
    let m1766 = {
        let a1763 = v0.clone();
        let a1764 = 0u64;
        let a1765 = 32u64;
        slice_bytes(a1763, a1764, a1765)
    };
    match m1766 {
        None => {
            v0
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f76(v0: Fn11, p1770: List<Bytes>, p1771: List<Adt3>) -> List<Bytes> {
    let m1769 = (p1770, p1771);
    {
        let mut m1769 = m1769;
        loop {
            let step = {
                let (v1, v2) = m1769;
                let m1767 = v2;
                match m1767.uncons() {
                    None => {
                        Err::<(List<Bytes>, List<Adt3>), List<Bytes>>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(List<Bytes>, List<Adt3>), List<Bytes>>(({
                            let c1768: Fn11 = v0.clone();
                            c1768.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m1769 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f77(v0: List<Bytes>, v1: Adt3) -> List<Bytes> {
    let m1772 = v1;
    match m1772 {
        Adt3::C0(v2) => {
            let a1774 = match v2.clone() {
                Adt4::C0(h1773, _, _, _, _, _, _, _, _, _, _, _) => {
                    h1773
                }
            };
            let a1775 = 2u64;
            if nat_eq(a1774, a1775) {
                List::cons(f82(match v2 {
                    Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1776) => {
                        h1776
                    }
                }), v0)
            } else {
                v0
            }
        }
        Adt3::C1(_, _) => {
            v0
        }
        Adt3::C2(_, _) => {
            v0
        }
    }
}

pub fn f78(v0: List<Bytes>, v1: Bytes) -> List<Bytes> {
    List::cons(v1, v0)
}

pub fn f79(v0: Adt4) -> Bytes {
    f5(List::cons(Bytes::lit(&[0x01]), List::cons(f2(match v0.clone() {
        Adt4::C0(h1777, _, _, _, _, _, _, _, _, _, _, _) => {
            h1777
        }
    }), List::cons(f3(match v0.clone() {
        Adt4::C0(_, h1778, _, _, _, _, _, _, _, _, _, _) => {
            h1778
        }
    }), List::cons(f3(match v0.clone() {
        Adt4::C0(_, _, h1779, _, _, _, _, _, _, _, _, _) => {
            h1779
        }
    }), List::cons(f3(match v0.clone() {
        Adt4::C0(_, _, _, h1780, _, _, _, _, _, _, _, _) => {
            h1780
        }
    }), List::cons(f3(match v0.clone() {
        Adt4::C0(_, _, _, _, h1781, _, _, _, _, _, _, _) => {
            h1781
        }
    }), List::cons(f82(f95(v0.clone())), List::cons(f82(match v0 {
        Adt4::C0(_, _, _, _, _, _, h1782, _, _, _, _, _) => {
            h1782
        }
    }), List::<Bytes>::nil())))))))))
}

pub fn f80(v0: u64, v1: Adt4) -> R<Bytes> {
    let v2: Adt13 = f96(match v1.clone() {
        Adt4::C0(_, _, _, _, _, h1783, _, _, _, _, _, _) => {
            h1783
        }
    }, match v1.clone() {
        Adt4::C0(_, _, _, _, _, _, h1784, _, _, _, _, _) => {
            h1784
        }
    }, match v1.clone() {
        Adt4::C0(_, _, _, _, _, _, _, h1785, _, _, _, _) => {
            h1785
        }
    }, match v1.clone() {
        Adt4::C0(_, _, _, _, _, _, _, _, h1786, _, _, _) => {
            h1786
        }
    }, match v1.clone() {
        Adt4::C0(_, _, _, _, _, _, _, _, _, h1787, _, _) => {
            h1787
        }
    }, match v1.clone() {
        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1788, _) => {
            h1788
        }
    })?;
    Ok(f5(List::cons(f3(match v1.clone() {
        Adt4::C0(_, _, _, _, h1789, _, _, _, _, _, _, _) => {
            h1789
        }
    }), List::cons(f12(match v1.clone() {
        Adt4::C0(_, _, _, _, _, _, _, _, _, h1790, _, _) => {
            h1790
        }
    }), List::cons(f82(match v1 {
        Adt4::C0(_, _, _, _, _, _, h1791, _, _, _, _, _) => {
            h1791
        }
    }), List::cons(Bytes::lit(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f]), f97(Fn14::F98(v0), List::cons(f82(f99(v2.clone())), List::<Bytes>::nil()), match v2 {
        Adt13::C0(_, _, _, _, _, _, h1792) => {
            h1792
        }
    })))))))
}

pub fn f81(v0: Adt5) -> Bytes {
    let a1813 = f14(match v0.clone() {
        Adt5::C0(h1793, _, _, _, _, _, _, _) => {
            h1793
        }
    });
    let a1814 = {
        let a1811 = f14(match v0.clone() {
            Adt5::C0(_, h1794, _, _, _, _, _, _) => {
                h1794
            }
        });
        let a1812 = {
            let a1809 = f14(match v0.clone() {
                Adt5::C0(_, _, h1795, _, _, _, _, _) => {
                    h1795
                }
            });
            let a1810 = {
                let a1807 = f14(match v0.clone() {
                    Adt5::C0(_, _, _, h1796, _, _, _, _) => {
                        h1796
                    }
                });
                let a1808 = {
                    let a1805 = f14(match v0.clone() {
                        Adt5::C0(_, _, _, _, h1797, _, _, _) => {
                            h1797
                        }
                    });
                    let a1806 = {
                        let a1803 = f14(match v0.clone() {
                            Adt5::C0(_, _, _, _, _, h1798, _, _) => {
                                h1798
                            }
                        });
                        let a1804 = {
                            let a1801 = f14(match v0.clone() {
                                Adt5::C0(_, _, _, _, _, _, h1799, _) => {
                                    h1799
                                }
                            });
                            let a1802 = f14(match v0 {
                                Adt5::C0(_, _, _, _, _, _, _, h1800) => {
                                    h1800
                                }
                            });
                            append_bytes(a1801, a1802)
                        };
                        append_bytes(a1803, a1804)
                    };
                    append_bytes(a1805, a1806)
                };
                append_bytes(a1807, a1808)
            };
            append_bytes(a1809, a1810)
        };
        append_bytes(a1811, a1812)
    };
    append_bytes(a1813, a1814)
}

pub fn f82(v0: Adt6) -> Bytes {
    let a1859 = f14(match v0.clone() {
        Adt6::C0(h1815, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1815
        }
    });
    let a1860 = {
        let a1857 = f14(match v0.clone() {
            Adt6::C0(_, h1816, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                h1816
            }
        });
        let a1858 = {
            let a1855 = f14(match v0.clone() {
                Adt6::C0(_, _, h1817, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                    h1817
                }
            });
            let a1856 = {
                let a1853 = f14(match v0.clone() {
                    Adt6::C0(_, _, _, h1818, _, _, _, _, _, _, _, _, _, _, _, _) => {
                        h1818
                    }
                });
                let a1854 = {
                    let a1851 = f14(match v0.clone() {
                        Adt6::C0(_, _, _, _, h1819, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1819
                        }
                    });
                    let a1852 = {
                        let a1849 = f14(match v0.clone() {
                            Adt6::C0(_, _, _, _, _, h1820, _, _, _, _, _, _, _, _, _, _) => {
                                h1820
                            }
                        });
                        let a1850 = {
                            let a1847 = f14(match v0.clone() {
                                Adt6::C0(_, _, _, _, _, _, h1821, _, _, _, _, _, _, _, _, _) => {
                                    h1821
                                }
                            });
                            let a1848 = {
                                let a1845 = f14(match v0.clone() {
                                    Adt6::C0(_, _, _, _, _, _, _, h1822, _, _, _, _, _, _, _, _) => {
                                        h1822
                                    }
                                });
                                let a1846 = {
                                    let a1843 = f14(match v0.clone() {
                                        Adt6::C0(_, _, _, _, _, _, _, _, h1823, _, _, _, _, _, _, _) => {
                                            h1823
                                        }
                                    });
                                    let a1844 = {
                                        let a1841 = f14(match v0.clone() {
                                            Adt6::C0(_, _, _, _, _, _, _, _, _, h1824, _, _, _, _, _, _) => {
                                                h1824
                                            }
                                        });
                                        let a1842 = {
                                            let a1839 = f14(match v0.clone() {
                                                Adt6::C0(_, _, _, _, _, _, _, _, _, _, h1825, _, _, _, _, _) => {
                                                    h1825
                                                }
                                            });
                                            let a1840 = {
                                                let a1837 = f14(match v0.clone() {
                                                    Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h1826, _, _, _, _) => {
                                                        h1826
                                                    }
                                                });
                                                let a1838 = {
                                                    let a1835 = f14(match v0.clone() {
                                                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1827, _, _, _) => {
                                                            h1827
                                                        }
                                                    });
                                                    let a1836 = {
                                                        let a1833 = f14(match v0.clone() {
                                                            Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1828, _, _) => {
                                                                h1828
                                                            }
                                                        });
                                                        let a1834 = {
                                                            let a1831 = f14(match v0.clone() {
                                                                Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1829, _) => {
                                                                    h1829
                                                                }
                                                            });
                                                            let a1832 = f14(match v0 {
                                                                Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1830) => {
                                                                    h1830
                                                                }
                                                            });
                                                            append_bytes(a1831, a1832)
                                                        };
                                                        append_bytes(a1833, a1834)
                                                    };
                                                    append_bytes(a1835, a1836)
                                                };
                                                append_bytes(a1837, a1838)
                                            };
                                            append_bytes(a1839, a1840)
                                        };
                                        append_bytes(a1841, a1842)
                                    };
                                    append_bytes(a1843, a1844)
                                };
                                append_bytes(a1845, a1846)
                            };
                            append_bytes(a1847, a1848)
                        };
                        append_bytes(a1849, a1850)
                    };
                    append_bytes(a1851, a1852)
                };
                append_bytes(a1853, a1854)
            };
            append_bytes(a1855, a1856)
        };
        append_bytes(a1857, a1858)
    };
    append_bytes(a1859, a1860)
}

pub fn f83(v0: Fn12, p1864: u64, p1865: (u64, Adt11)) -> R<(u64, Adt11)> {
    let m1863 = (p1864, p1865);
    {
        let mut m1863 = m1863;
        loop {
            let step = {
                let (v1, v2) = m1863;
                let m1861 = v1;
                if m1861 == 0 {
                    Err::<(u64, (u64, Adt11)), R<(u64, Adt11)>>(Ok(v2))
                } else {
                    let v3 = m1861 - 1;
                    Ok::<(u64, (u64, Adt11)), R<(u64, Adt11)>>((v3, {
                        let c1862: Fn12 = v0.clone();
                        c1862.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m1863 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f84(v0: Bytes, v1: u64, v2: u64, v3: (u64, Adt11)) -> R<(u64, Adt11)> {
    Ok(({
        let a1867 = {
            let (h1866, _) = v3.clone();
            h1866
        };
        let a1868 = 64u64;
        nat_add(a1867, a1868)?
    }, f100({
        let (_, h1869) = v3;
        h1869
    }, f101(v0, v1, v2, {
        let (h1870, _) = v3;
        h1870
    })?)?))
}

pub fn f85() -> Adt11 {
    Adt11::C0(1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, 1359893119u32, 2600822924u32, 528734635u32, 1541459225u32)
}

pub fn f86(v0: Bytes, v1: u64, v2: u64) -> R<u32> {
    Ok(f102(f103(v0.clone(), v1, v2), f103(v0.clone(), v1, {
        let a1871 = v2;
        let a1872 = 1u64;
        nat_add(a1871, a1872)?
    }), f103(v0.clone(), v1, {
        let a1873 = v2;
        let a1874 = 2u64;
        nat_add(a1873, a1874)?
    }), f103(v0, v1, {
        let a1875 = v2;
        let a1876 = 3u64;
        nat_add(a1875, a1876)?
    })))
}

pub fn f87(v0: Fn13, p1880: Adt6, p1881: List<Adt3>) -> Adt6 {
    let m1879 = (p1880, p1881);
    {
        let mut m1879 = m1879;
        loop {
            let step = {
                let (v1, v2) = m1879;
                let m1877 = v2;
                match m1877.uncons() {
                    None => {
                        Err::<(Adt6, List<Adt3>), Adt6>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt6, List<Adt3>), Adt6>(({
                            let c1878: Fn13 = v0.clone();
                            c1878.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m1879 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f88(v0: u64, v1: Adt6, v2: Adt3) -> Adt6 {
    let m1882 = v2;
    match m1882 {
        Adt3::C0(v3) => {
            let a1884 = match v3.clone() {
                Adt4::C0(h1883, _, _, _, _, _, _, _, _, _, _, _) => {
                    h1883
                }
            };
            let a1885 = 2u64;
            if nat_eq(a1884, a1885) {
                let a1887 = match v3.clone() {
                    Adt4::C0(_, _, _, h1886, _, _, _, _, _, _, _, _) => {
                        h1886
                    }
                };
                let a1888 = v0;
                if nat_eq(a1887, a1888) {
                    match v3 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1889) => {
                            h1889
                        }
                    }
                } else {
                    v1
                }
            } else {
                v1
            }
        }
        Adt3::C1(_, _) => {
            v1
        }
        Adt3::C2(_, _) => {
            v1
        }
    }
}

pub fn f89() -> u32 {
    1u32
}

pub fn f90(v0: Adt5, v1: Adt6, v2: u32, v3: u32, v4: u32, v5: u32) -> R<Adt6> {
    Ok(f99(f96(v0, v1, v2, v3, v4, v5)?))
}

pub fn f91() -> u32 {
    4u32
}

pub fn f92() -> u32 {
    8u32
}

pub fn f93(v0: Adt5, v1: u32, v2: Adt12) -> R<(List<Adt7>, List<Adt3>)> {
    let v3: Adt12 = {
        let (h1890, _) = f104(Fn15::F105(v1, v0), 64u64, v2)?;
        h1890
    };
    let v4: List<Adt7> = List::cons(match v3.clone() {
        Adt12::C0(h1891, _, _, _) => {
            h1891
        }
    }, match v3.clone() {
        Adt12::C0(_, _, h1892, _) => {
            h1892
        }
    });
    Ok((v4.clone(), List::cons(Adt3::C1(match v3.clone() {
        Adt12::C0(h1893, _, _, _) => {
            h1893
        }
    }, {
        let a1894 = v4;
        length_list(a1894)
    }), match v3 {
        Adt12::C0(_, _, _, h1895) => {
            h1895
        }
    })))
}

pub fn f94(v0: u64) -> Bytes {
    f14(f13(v0, 32u64))
}

pub fn f95(v0: Adt4) -> Adt6 {
    f106(match v0.clone() {
        Adt4::C0(_, _, _, _, _, h1896, _, _, _, _, _, _) => {
            h1896
        }
    }, match v0.clone() {
        Adt4::C0(_, _, _, _, _, _, _, h1897, _, _, _, _) => {
            h1897
        }
    }, match v0.clone() {
        Adt4::C0(_, _, _, _, _, _, _, _, h1898, _, _, _) => {
            h1898
        }
    }, match v0.clone() {
        Adt4::C0(_, _, _, _, _, _, _, _, _, h1899, _, _) => {
            h1899
        }
    }, match v0 {
        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1900, _) => {
            h1900
        }
    })
}

pub fn f96(v0: Adt5, v1: Adt6, v2: u32, v3: u32, v4: u32, v5: u32) -> R<Adt13> {
    let v6: (u64, Adt16) = f107(Fn16::F108(v0.clone(), v1.clone()), 800u64, (0u64, Adt16::C0(f106(v0.clone(), v2, v3, v4, v5), Adt6::C0(0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32), List::<Adt14>::nil())))?;
    Ok(Adt13::C0(v0, v1, v2, v3, v4, v5, {
        let m1903 = {
            let (_, h1902) = v6;
            h1902
        };
        match m1903 {
            Adt16::C0(_, _, h1901) => {
                h1901
            }
        }
    }))
}

pub fn f97(v0: Fn14, p1907: List<Bytes>, p1908: List<Adt14>) -> List<Bytes> {
    let m1906 = (p1907, p1908);
    {
        let mut m1906 = m1906;
        loop {
            let step = {
                let (v1, v2) = m1906;
                let m1904 = v2;
                match m1904.uncons() {
                    None => {
                        Err::<(List<Bytes>, List<Adt14>), List<Bytes>>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(List<Bytes>, List<Adt14>), List<Bytes>>(({
                            let c1905: Fn14 = v0.clone();
                            c1905.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m1906 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f98(v0: u64, v1: List<Bytes>, v2: Adt14) -> List<Bytes> {
    List::cons(if f109(v2.clone()) {
        let a1910 = f110(match v2.clone() {
            Adt14::C0(h1909, _, _, _, _, _, _, _, _) => {
                h1909
            }
        });
        let a1911 = f111(v0, v2);
        append_bytes(a1910, a1911)
    } else {
        f111(v0, v2)
    }, v1)
}

pub fn f99(v0: Adt13) -> Adt6 {
    f112(Fn17::F113, Adt6::C0(0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32), match v0 {
        Adt13::C0(_, _, _, _, _, _, h1912) => {
            h1912
        }
    })
}

pub fn f100(v0: Adt11, v1: Adt17) -> R<Adt11> {
    let v2: Adt18 = f114(Fn18::F115, 64u64, Adt18::C0(match v0.clone() {
        Adt11::C0(h1913, _, _, _, _, _, _, _) => {
            h1913
        }
    }, match v0.clone() {
        Adt11::C0(_, h1914, _, _, _, _, _, _) => {
            h1914
        }
    }, match v0.clone() {
        Adt11::C0(_, _, h1915, _, _, _, _, _) => {
            h1915
        }
    }, match v0.clone() {
        Adt11::C0(_, _, _, h1916, _, _, _, _) => {
            h1916
        }
    }, match v0.clone() {
        Adt11::C0(_, _, _, _, h1917, _, _, _) => {
            h1917
        }
    }, match v0.clone() {
        Adt11::C0(_, _, _, _, _, h1918, _, _) => {
            h1918
        }
    }, match v0.clone() {
        Adt11::C0(_, _, _, _, _, _, h1919, _) => {
            h1919
        }
    }, match v0.clone() {
        Adt11::C0(_, _, _, _, _, _, _, h1920) => {
            h1920
        }
    }, v1, 0u64))?;
    Ok(Adt11::C0(f116(match v0.clone() {
        Adt11::C0(h1921, _, _, _, _, _, _, _) => {
            h1921
        }
    }, match v2.clone() {
        Adt18::C0(h1922, _, _, _, _, _, _, _, _, _) => {
            h1922
        }
    }), f116(match v0.clone() {
        Adt11::C0(_, h1923, _, _, _, _, _, _) => {
            h1923
        }
    }, match v2.clone() {
        Adt18::C0(_, h1924, _, _, _, _, _, _, _, _) => {
            h1924
        }
    }), f116(match v0.clone() {
        Adt11::C0(_, _, h1925, _, _, _, _, _) => {
            h1925
        }
    }, match v2.clone() {
        Adt18::C0(_, _, h1926, _, _, _, _, _, _, _) => {
            h1926
        }
    }), f116(match v0.clone() {
        Adt11::C0(_, _, _, h1927, _, _, _, _) => {
            h1927
        }
    }, match v2.clone() {
        Adt18::C0(_, _, _, h1928, _, _, _, _, _, _) => {
            h1928
        }
    }), f116(match v0.clone() {
        Adt11::C0(_, _, _, _, h1929, _, _, _) => {
            h1929
        }
    }, match v2.clone() {
        Adt18::C0(_, _, _, _, h1930, _, _, _, _, _) => {
            h1930
        }
    }), f116(match v0.clone() {
        Adt11::C0(_, _, _, _, _, h1931, _, _) => {
            h1931
        }
    }, match v2.clone() {
        Adt18::C0(_, _, _, _, _, h1932, _, _, _, _) => {
            h1932
        }
    }), f116(match v0.clone() {
        Adt11::C0(_, _, _, _, _, _, h1933, _) => {
            h1933
        }
    }, match v2.clone() {
        Adt18::C0(_, _, _, _, _, _, h1934, _, _, _) => {
            h1934
        }
    }), f116(match v0 {
        Adt11::C0(_, _, _, _, _, _, _, h1935) => {
            h1935
        }
    }, match v2 {
        Adt18::C0(_, _, _, _, _, _, _, h1936, _, _) => {
            h1936
        }
    })))
}

pub fn f101(v0: Bytes, v1: u64, v2: u64, v3: u64) -> R<Adt17> {
    Ok(Adt17::C0(f117(v0.clone(), v1, v2, v3)?, f117(v0.clone(), v1, v2, {
        let a1937 = v3;
        let a1938 = 4u64;
        nat_add(a1937, a1938)?
    })?, f117(v0.clone(), v1, v2, {
        let a1939 = v3;
        let a1940 = 8u64;
        nat_add(a1939, a1940)?
    })?, f117(v0.clone(), v1, v2, {
        let a1941 = v3;
        let a1942 = 12u64;
        nat_add(a1941, a1942)?
    })?, f117(v0.clone(), v1, v2, {
        let a1943 = v3;
        let a1944 = 16u64;
        nat_add(a1943, a1944)?
    })?, f117(v0.clone(), v1, v2, {
        let a1945 = v3;
        let a1946 = 20u64;
        nat_add(a1945, a1946)?
    })?, f117(v0.clone(), v1, v2, {
        let a1947 = v3;
        let a1948 = 24u64;
        nat_add(a1947, a1948)?
    })?, f117(v0.clone(), v1, v2, {
        let a1949 = v3;
        let a1950 = 28u64;
        nat_add(a1949, a1950)?
    })?, f117(v0.clone(), v1, v2, {
        let a1951 = v3;
        let a1952 = 32u64;
        nat_add(a1951, a1952)?
    })?, f117(v0.clone(), v1, v2, {
        let a1953 = v3;
        let a1954 = 36u64;
        nat_add(a1953, a1954)?
    })?, f117(v0.clone(), v1, v2, {
        let a1955 = v3;
        let a1956 = 40u64;
        nat_add(a1955, a1956)?
    })?, f117(v0.clone(), v1, v2, {
        let a1957 = v3;
        let a1958 = 44u64;
        nat_add(a1957, a1958)?
    })?, f117(v0.clone(), v1, v2, {
        let a1959 = v3;
        let a1960 = 48u64;
        nat_add(a1959, a1960)?
    })?, f117(v0.clone(), v1, v2, {
        let a1961 = v3;
        let a1962 = 52u64;
        nat_add(a1961, a1962)?
    })?, f117(v0.clone(), v1, v2, {
        let a1963 = v3;
        let a1964 = 56u64;
        nat_add(a1963, a1964)?
    })?, f117(v0, v1, v2, {
        let a1965 = v3;
        let a1966 = 60u64;
        nat_add(a1965, a1966)?
    })?))
}

pub fn f102(v0: u32, v1: u32, v2: u32, v3: u32) -> u32 {
    let a1971 = v0;
    let a1972 = {
        let a1969 = f52(v1, 8u32);
        let a1970 = {
            let a1967 = f52(v2, 16u32);
            let a1968 = f52(v3, 24u32);
            fixed_u32::bit_or(a1967, a1968)
        };
        fixed_u32::bit_or(a1969, a1970)
    };
    fixed_u32::bit_or(a1971, a1972)
}

pub fn f103(v0: Bytes, v1: u64, v2: u64) -> u32 {
    let a1973 = v2;
    let a1974 = v1;
    if nat_lt(a1973, a1974) {
        let m1977 = {
            let a1975 = v0;
            let a1976 = v2;
            index_bytes(a1975, a1976)
        };
        match m1977 {
            None => {
                0u32
            }
            Some(v3) => {
                let m1979 = {
                    let a1978 = v3;
                    fixed_u32::convert(i128::from(a1978))
                };
                match m1979 {
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

pub fn f104(v0: Fn15, p1984: u64, p1985: Adt12) -> R<(Adt12, bool)> {
    let m1983 = (p1984, p1985);
    {
        let mut m1983 = m1983;
        loop {
            let step = {
                let (v1, v2) = m1983;
                let m1980 = v1;
                if m1980 == 0 {
                    Err::<(u64, Adt12), R<(Adt12, bool)>>(Ok((v2, false)))
                } else {
                    let v3 = m1980 - 1;
                    let m1982 = {
                        let c1981: Fn15 = v0.clone();
                        c1981.apply(v2.clone())?
                    };
                    match m1982 {
                        None => {
                            Err::<(u64, Adt12), R<(Adt12, bool)>>(Ok((v2, true)))
                        }
                        Some(v4) => {
                            Ok::<(u64, Adt12), R<(Adt12, bool)>>((v3, v4))
                        }
                    }
                }
            };
            match step {
                Ok(next) => { m1983 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f105(v0: u32, v1: Adt5, v2: Adt12) -> R<Option<Adt12>> {
    f118(v1, v0, v2)
}

pub fn f106(v0: Adt5, v1: u32, v2: u32, v3: u32, v4: u32) -> Adt6 {
    Adt6::C0(match v0.clone() {
        Adt5::C0(h1986, _, _, _, _, _, _, _) => {
            h1986
        }
    }, match v0.clone() {
        Adt5::C0(_, h1987, _, _, _, _, _, _) => {
            h1987
        }
    }, match v0.clone() {
        Adt5::C0(_, _, h1988, _, _, _, _, _) => {
            h1988
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, h1989, _, _, _, _) => {
            h1989
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, h1990, _, _, _) => {
            h1990
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, h1991, _, _) => {
            h1991
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, h1992, _) => {
            h1992
        }
    }, match v0 {
        Adt5::C0(_, _, _, _, _, _, _, h1993) => {
            h1993
        }
    }, 1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, v1, v2, v3, v4)
}

pub fn f107(v0: Fn16, p1997: u64, p1998: (u64, Adt16)) -> R<(u64, Adt16)> {
    let m1996 = (p1997, p1998);
    {
        let mut m1996 = m1996;
        loop {
            let step = {
                let (v1, v2) = m1996;
                let m1994 = v1;
                if m1994 == 0 {
                    Err::<(u64, (u64, Adt16)), R<(u64, Adt16)>>(Ok(v2))
                } else {
                    let v3 = m1994 - 1;
                    Ok::<(u64, (u64, Adt16)), R<(u64, Adt16)>>((v3, {
                        let c1995: Fn16 = v0.clone();
                        c1995.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m1996 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f108(v0: Adt5, v1: Adt6, v2: (u64, Adt16)) -> R<(u64, Adt16)> {
    Ok(({
        let a2000 = {
            let (h1999, _) = v2.clone();
            h1999
        };
        let a2001 = 1u64;
        nat_add(a2000, a2001)?
    }, f119(v0, v1, {
        let (_, h2002) = v2;
        h2002
    }, {
        let (h2003, _) = v2;
        h2003
    })?))
}

pub fn f109(v0: Adt14) -> bool {
    let a2005 = match v0.clone() {
        Adt14::C0(h2004, _, _, _, _, _, _, _, _) => {
            h2004
        }
    };
    let a2006 = 7u64;
    if nat_lt(a2005, a2006) {
        let a2008 = match v0.clone() {
            Adt14::C0(_, h2007, _, _, _, _, _, _, _) => {
                h2007
            }
        };
        let a2009 = 0u64;
        if nat_eq(a2008, a2009) {
            let a2011 = match v0 {
                Adt14::C0(_, _, h2010, _, _, _, _, _, _) => {
                    h2010
                }
            };
            let a2012 = 0u64;
            nat_eq(a2011, a2012)
        } else {
            false
        }
    } else {
        false
    }
}

pub fn f110(v0: u64) -> Bytes {
    let a2013 = v0;
    let a2014 = 3u64;
    if nat_lt(a2013, a2014) {
        let a2015 = v0;
        let a2016 = 1u64;
        if nat_lt(a2015, a2016) {
            Bytes::lit(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f])
        } else {
            let a2017 = v0;
            let a2018 = 2u64;
            if nat_lt(a2017, a2018) {
                Bytes::lit(&[0x02, 0x06, 0x03, 0x0a, 0x07, 0x00, 0x04, 0x0d, 0x01, 0x0b, 0x0c, 0x05, 0x09, 0x0e, 0x0f, 0x08])
            } else {
                Bytes::lit(&[0x03, 0x04, 0x0a, 0x0c, 0x0d, 0x02, 0x07, 0x0e, 0x06, 0x05, 0x09, 0x00, 0x0b, 0x0f, 0x08, 0x01])
            }
        }
    } else {
        let a2019 = v0;
        let a2020 = 5u64;
        if nat_lt(a2019, a2020) {
            let a2021 = v0;
            let a2022 = 4u64;
            if nat_lt(a2021, a2022) {
                Bytes::lit(&[0x0a, 0x07, 0x0c, 0x09, 0x0e, 0x03, 0x0d, 0x0f, 0x04, 0x00, 0x0b, 0x02, 0x05, 0x08, 0x01, 0x06])
            } else {
                Bytes::lit(&[0x0c, 0x0d, 0x09, 0x0b, 0x0f, 0x0a, 0x0e, 0x08, 0x07, 0x02, 0x05, 0x03, 0x00, 0x01, 0x06, 0x04])
            }
        } else {
            let a2023 = v0;
            let a2024 = 6u64;
            if nat_lt(a2023, a2024) {
                Bytes::lit(&[0x09, 0x0e, 0x0b, 0x05, 0x08, 0x0c, 0x0f, 0x01, 0x0d, 0x03, 0x00, 0x0a, 0x02, 0x06, 0x04, 0x07])
            } else {
                Bytes::lit(&[0x0b, 0x0f, 0x05, 0x00, 0x01, 0x09, 0x08, 0x06, 0x0e, 0x0a, 0x02, 0x0c, 0x03, 0x04, 0x07, 0x0d])
            }
        }
    }
}

pub fn f111(v0: u64, v1: Adt14) -> Bytes {
    f5(List::cons(f2(match v1.clone() {
        Adt14::C0(h2025, _, _, _, _, _, _, _, _) => {
            h2025
        }
    }), List::cons(f2(match v1.clone() {
        Adt14::C0(_, h2026, _, _, _, _, _, _, _) => {
            h2026
        }
    }), List::cons(f2(match v1.clone() {
        Adt14::C0(_, _, h2027, _, _, _, _, _, _) => {
            h2027
        }
    }), List::cons(f2(f120(match v1.clone() {
        Adt14::C0(_, _, _, h2028, _, _, _, _, _) => {
            h2028
        }
    })), List::cons(f2(match v1.clone() {
        Adt14::C0(_, _, _, _, h2029, _, _, _, _) => {
            h2029
        }
    }), List::cons(f2(match v1.clone() {
        Adt14::C0(_, _, _, _, _, h2030, _, _, _) => {
            h2030
        }
    }), List::cons(f14(match v1.clone() {
        Adt14::C0(_, _, _, _, _, _, h2031, _, _) => {
            h2031
        }
    }), List::cons(f14(match v1.clone() {
        Adt14::C0(_, _, _, _, _, _, _, h2032, _) => {
            h2032
        }
    }), List::cons(f14(match v1.clone() {
        Adt14::C0(_, _, _, _, _, _, _, _, h2033) => {
            h2033
        }
    }), List::cons(f121(v0, v1), List::<Bytes>::nil())))))))))))
}

pub fn f112(v0: Fn17, p2037: Adt6, p2038: List<Adt14>) -> Adt6 {
    let m2036 = (p2037, p2038);
    {
        let mut m2036 = m2036;
        loop {
            let step = {
                let (v1, v2) = m2036;
                let m2034 = v2;
                match m2034.uncons() {
                    None => {
                        Err::<(Adt6, List<Adt14>), Adt6>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt6, List<Adt14>), Adt6>(({
                            let c2035: Fn17 = v0.clone();
                            c2035.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m2036 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f113(v0: Adt6, v1: Adt14) -> Adt6 {
    let a2040 = match v1.clone() {
        Adt14::C0(h2039, _, _, _, _, _, _, _, _) => {
            h2039
        }
    };
    let a2041 = 7u64;
    if nat_eq(a2040, a2041) {
        f122(v0, match v1.clone() {
            Adt14::C0(_, _, _, _, h2042, _, _, _, _) => {
                h2042
            }
        }, match v1 {
            Adt14::C0(_, _, _, _, _, _, _, _, h2043) => {
                h2043
            }
        })
    } else {
        v0
    }
}

pub fn f114(v0: Fn18, p2047: u64, p2048: Adt18) -> R<Adt18> {
    let m2046 = (p2047, p2048);
    {
        let mut m2046 = m2046;
        loop {
            let step = {
                let (v1, v2) = m2046;
                let m2044 = v1;
                if m2044 == 0 {
                    Err::<(u64, Adt18), R<Adt18>>(Ok(v2))
                } else {
                    let v3 = m2044 - 1;
                    Ok::<(u64, Adt18), R<Adt18>>((v3, {
                        let c2045: Fn18 = v0.clone();
                        c2045.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m2046 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f115(v0: Adt18) -> R<Adt18> {
    f123(v0)
}

pub fn f116(v0: u32, v1: u32) -> u32 {
    let m2051 = {
        let a2049 = f124(v0);
        let a2050 = f124(v1);
        fixed_u64::checked_add(a2049, a2050)
    };
    match m2051 {
        None => {
            0u32
        }
        Some(v2) => {
            f125(v2)
        }
    }
}

pub fn f117(v0: Bytes, v1: u64, v2: u64, v3: u64) -> R<u32> {
    let a2054 = {
        let a2052 = v3;
        let a2053 = 8u64;
        nat_add(a2052, a2053)?
    };
    let a2055 = v2;
    if nat_eq(a2054, a2055) {
        Ok(f126({
            let a2058 = {
                let a2056 = v1;
                let a2057 = 8u64;
                nat_mul(a2056, a2057)?
            };
            let a2059 = 4294967296u64;
            let a2060 = 0u64;
            nat_quot(a2058, a2059, a2060)
        }))
    } else {
        let a2063 = {
            let a2061 = v3;
            let a2062 = 4u64;
            nat_add(a2061, a2062)?
        };
        let a2064 = v2;
        if nat_eq(a2063, a2064) {
            Ok(f126({
                let a2067 = {
                    let a2065 = v1;
                    let a2066 = 8u64;
                    nat_mul(a2065, a2066)?
                };
                let a2068 = 4294967296u64;
                let a2069 = 0u64;
                nat_rem(a2067, a2068, a2069)
            }))
        } else {
            Ok(f127(f128(v0.clone(), v1, v3), f128(v0.clone(), v1, {
                let a2070 = v3;
                let a2071 = 1u64;
                nat_add(a2070, a2071)?
            }), f128(v0.clone(), v1, {
                let a2072 = v3;
                let a2073 = 2u64;
                nat_add(a2072, a2073)?
            }), f128(v0, v1, {
                let a2074 = v3;
                let a2075 = 3u64;
                nat_add(a2074, a2075)?
            })))
        }
    }
}

pub fn f118(v0: Adt5, v1: u32, v2: Adt12) -> R<Option<Adt12>> {
    let a2080 = {
        let a2077 = match v2.clone() {
            Adt12::C0(_, h2076, _, _) => {
                h2076
            }
        };
        let a2078 = 2u64;
        let a2079 = 0u64;
        nat_rem(a2077, a2078, a2079)
    };
    let a2081 = 0u64;
    if nat_eq(a2080, a2081) {
        let m2083 = match v2.clone() {
            Adt12::C0(_, _, h2082, _) => {
                h2082
            }
        };
        match m2083.uncons() {
            None => {
                Ok(None::<Adt12>)
            }
            Some((v3, v4)) => {
                let v5: Adt4 = f67(f68(v3.clone(), match v2.clone() {
                    Adt12::C0(h2084, _, _, _) => {
                        h2084
                    }
                }, v0, v1)?)?;
                Ok(Some(Adt12::C0(Adt7::C0(f56(match v5.clone() {
                    Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h2085) => {
                        h2085
                    }
                }), match v3.clone() {
                    Adt7::C0(_, h2086, _) => {
                        h2086
                    }
                }, {
                    let a2091 = match v3.clone() {
                        Adt7::C0(_, _, h2087) => {
                            h2087
                        }
                    };
                    let a2092 = {
                        let m2090 = match v2.clone() {
                            Adt12::C0(h2089, _, _, _) => {
                                h2089
                            }
                        };
                        match m2090 {
                            Adt7::C0(_, _, h2088) => {
                                h2088
                            }
                        }
                    };
                    nat_add(a2091, a2092)?
                }), {
                    let a2094 = match v2.clone() {
                        Adt12::C0(_, h2093, _, _) => {
                            h2093
                        }
                    };
                    let a2095 = 2u64;
                    let a2096 = 0u64;
                    nat_quot(a2094, a2095, a2096)
                }, v4, List::cons(Adt3::C0(v5), List::cons(Adt3::C2(v3, {
                    let a2098 = match v2.clone() {
                        Adt12::C0(_, _, h2097, _) => {
                            h2097
                        }
                    };
                    length_list(a2098)
                }), match v2 {
                    Adt12::C0(_, _, _, h2099) => {
                        h2099
                    }
                })))))
            }
        }
    } else {
        Ok(None::<Adt12>)
    }
}

pub fn f119(v0: Adt5, v1: Adt6, v2: Adt16, v3: u64) -> R<Adt16> {
    let v4: u64 = {
        let a2100 = v3;
        let a2101 = 784u64;
        if nat_lt(a2100, a2101) {
            let a2102 = v3;
            let a2103 = 112u64;
            let a2104 = 0u64;
            nat_quot(a2102, a2103, a2104)
        } else {
            7u64
        }
    };
    let v5: Adt19 = f129({
        let a2105 = v3;
        let a2106 = 784u64;
        if nat_lt(a2105, a2106) {
            let a2107 = v3;
            let a2108 = 112u64;
            let a2109 = 0u64;
            nat_rem(a2107, a2108, a2109)
        } else {
            let a2110 = v3;
            let a2111 = 672u64;
            nat_sub(a2110, a2111)
        }
    });
    let v6: Adt6 = match v2.clone() {
        Adt16::C0(h2112, _, _) => {
            h2112
        }
    };
    let v7: u32 = f130(v6.clone(), match v5.clone() {
        Adt19::C0(_, _, _, h2113, _) => {
            h2113
        }
    });
    let v8: u32 = {
        let m2115 = match v5.clone() {
            Adt19::C0(_, _, h2114, _, _) => {
                h2114
            }
        };
        match m2115 {
            Adt15::C0 => {
                f130(v6.clone(), match v5.clone() {
                    Adt19::C0(_, _, _, _, h2116) => {
                        h2116
                    }
                })
            }
            Adt15::C1 => {
                f130(v1, f131(v4, match v5.clone() {
                    Adt19::C0(_, _, _, _, h2117) => {
                        h2117
                    }
                })?)
            }
            Adt15::C2 => {
                f130(v6.clone(), match v5.clone() {
                    Adt19::C0(_, _, _, _, h2118) => {
                        h2118
                    }
                })
            }
            Adt15::C3 => {
                f132(match v5.clone() {
                    Adt19::C0(_, _, _, _, h2119) => {
                        h2119
                    }
                })
            }
            Adt15::C4 => {
                f133(v0, match v5.clone() {
                    Adt19::C0(_, _, _, _, h2120) => {
                        h2120
                    }
                })
            }
        }
    };
    let v9: u32 = f134(match v5.clone() {
        Adt19::C0(_, _, h2121, _, _) => {
            h2121
        }
    }, v7, v8);
    Ok(Adt16::C0({
        let a2122 = v4;
        let a2123 = 7u64;
        if nat_eq(a2122, a2123) {
            v6
        } else {
            f122(v6, match v5.clone() {
                Adt19::C0(_, _, _, h2124, _) => {
                    h2124
                }
            }, v9)
        }
    }, {
        let a2125 = v4;
        let a2126 = 7u64;
        if nat_eq(a2125, a2126) {
            f122(match v2.clone() {
                Adt16::C0(_, h2127, _) => {
                    h2127
                }
            }, match v5.clone() {
                Adt19::C0(_, _, _, h2128, _) => {
                    h2128
                }
            }, v9)
        } else {
            match v2.clone() {
                Adt16::C0(_, h2129, _) => {
                    h2129
                }
            }
        }
    }, List::cons(Adt14::C0(v4, match v5.clone() {
        Adt19::C0(h2130, _, _, _, _) => {
            h2130
        }
    }, match v5.clone() {
        Adt19::C0(_, h2131, _, _, _) => {
            h2131
        }
    }, match v5.clone() {
        Adt19::C0(_, _, h2132, _, _) => {
            h2132
        }
    }, match v5.clone() {
        Adt19::C0(_, _, _, h2133, _) => {
            h2133
        }
    }, match v5 {
        Adt19::C0(_, _, _, _, h2134) => {
            h2134
        }
    }, v7, v8, v9), match v2 {
        Adt16::C0(_, _, h2135) => {
            h2135
        }
    })))
}

pub fn f120(v0: Adt15) -> u64 {
    let m2136 = v0;
    match m2136 {
        Adt15::C0 => {
            0u64
        }
        Adt15::C1 => {
            1u64
        }
        Adt15::C2 => {
            2u64
        }
        Adt15::C3 => {
            3u64
        }
        Adt15::C4 => {
            4u64
        }
    }
}

pub fn f121(v0: u64, v1: Adt14) -> Bytes {
    let a2137 = v0;
    let a2138 = 2u64;
    if nat_lt(a2137, a2138) {
        Bytes::lit(&[])
    } else {
        let m2140 = match v1.clone() {
            Adt14::C0(_, _, _, h2139, _, _, _, _, _) => {
                h2139
            }
        };
        match m2140 {
            Adt15::C0 => {
                f135(match v1.clone() {
                    Adt14::C0(_, _, _, _, _, _, h2141, _, _) => {
                        h2141
                    }
                }, match v1 {
                    Adt14::C0(_, _, _, _, _, _, _, h2142, _) => {
                        h2142
                    }
                })
            }
            Adt15::C1 => {
                f135(match v1.clone() {
                    Adt14::C0(_, _, _, _, _, _, h2143, _, _) => {
                        h2143
                    }
                }, match v1 {
                    Adt14::C0(_, _, _, _, _, _, _, h2144, _) => {
                        h2144
                    }
                })
            }
            Adt15::C2 => {
                f136(match v1.clone() {
                    Adt14::C0(_, _, _, _, _, _, h2145, _, _) => {
                        h2145
                    }
                }, match v1 {
                    Adt14::C0(_, _, _, _, _, _, _, h2146, _) => {
                        h2146
                    }
                })
            }
            Adt15::C3 => {
                Bytes::lit(&[])
            }
            Adt15::C4 => {
                f136(match v1.clone() {
                    Adt14::C0(_, _, _, _, _, _, h2147, _, _) => {
                        h2147
                    }
                }, match v1 {
                    Adt14::C0(_, _, _, _, _, _, _, h2148, _) => {
                        h2148
                    }
                })
            }
        }
    }
}

pub fn f122(v0: Adt6, v1: u64, v2: u32) -> Adt6 {
    let a2149 = v1;
    let a2150 = 8u64;
    if nat_lt(a2149, a2150) {
        let a2151 = v1;
        let a2152 = 4u64;
        if nat_lt(a2151, a2152) {
            let a2153 = v1;
            let a2154 = 2u64;
            if nat_lt(a2153, a2154) {
                let a2155 = v1;
                let a2156 = 1u64;
                if nat_lt(a2155, a2156) {
                    Adt6::C0(v2, match v0.clone() {
                        Adt6::C0(_, h2157, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2157
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2158, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2158
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2159, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2159
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2160, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2160
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2161, _, _, _, _, _, _, _, _, _, _) => {
                            h2161
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2162, _, _, _, _, _, _, _, _, _) => {
                            h2162
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2163, _, _, _, _, _, _, _, _) => {
                            h2163
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2164, _, _, _, _, _, _, _) => {
                            h2164
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2165, _, _, _, _, _, _) => {
                            h2165
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2166, _, _, _, _, _) => {
                            h2166
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2167, _, _, _, _) => {
                            h2167
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2168, _, _, _) => {
                            h2168
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2169, _, _) => {
                            h2169
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2170, _) => {
                            h2170
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2171) => {
                            h2171
                        }
                    })
                } else {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2172, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2172
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, h2173, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2173
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2174, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2174
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2175, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2175
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2176, _, _, _, _, _, _, _, _, _, _) => {
                            h2176
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2177, _, _, _, _, _, _, _, _, _) => {
                            h2177
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2178, _, _, _, _, _, _, _, _) => {
                            h2178
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2179, _, _, _, _, _, _, _) => {
                            h2179
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2180, _, _, _, _, _, _) => {
                            h2180
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2181, _, _, _, _, _) => {
                            h2181
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2182, _, _, _, _) => {
                            h2182
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2183, _, _, _) => {
                            h2183
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2184, _, _) => {
                            h2184
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2185, _) => {
                            h2185
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2186) => {
                            h2186
                        }
                    })
                }
            } else {
                let a2187 = v1;
                let a2188 = 3u64;
                if nat_lt(a2187, a2188) {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2189, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2189
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2190, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2190
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, h2191, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2191
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2192, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2192
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2193, _, _, _, _, _, _, _, _, _, _) => {
                            h2193
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2194, _, _, _, _, _, _, _, _, _) => {
                            h2194
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2195, _, _, _, _, _, _, _, _) => {
                            h2195
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2196, _, _, _, _, _, _, _) => {
                            h2196
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2197, _, _, _, _, _, _) => {
                            h2197
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2198, _, _, _, _, _) => {
                            h2198
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2199, _, _, _, _) => {
                            h2199
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2200, _, _, _) => {
                            h2200
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2201, _, _) => {
                            h2201
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2202, _) => {
                            h2202
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2203) => {
                            h2203
                        }
                    })
                } else {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2204, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2204
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2205, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2205
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2206, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2206
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2207, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2207
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2208, _, _, _, _, _, _, _, _, _, _) => {
                            h2208
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2209, _, _, _, _, _, _, _, _, _) => {
                            h2209
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2210, _, _, _, _, _, _, _, _) => {
                            h2210
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2211, _, _, _, _, _, _, _) => {
                            h2211
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2212, _, _, _, _, _, _) => {
                            h2212
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2213, _, _, _, _, _) => {
                            h2213
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2214, _, _, _, _) => {
                            h2214
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2215, _, _, _) => {
                            h2215
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2216, _, _) => {
                            h2216
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2217, _) => {
                            h2217
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2218) => {
                            h2218
                        }
                    })
                }
            }
        } else {
            let a2219 = v1;
            let a2220 = 6u64;
            if nat_lt(a2219, a2220) {
                let a2221 = v1;
                let a2222 = 5u64;
                if nat_lt(a2221, a2222) {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2223, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2223
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2224, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2224
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2225, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2225
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2226, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2226
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2227, _, _, _, _, _, _, _, _, _, _) => {
                            h2227
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2228, _, _, _, _, _, _, _, _, _) => {
                            h2228
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2229, _, _, _, _, _, _, _, _) => {
                            h2229
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2230, _, _, _, _, _, _, _) => {
                            h2230
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2231, _, _, _, _, _, _) => {
                            h2231
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2232, _, _, _, _, _) => {
                            h2232
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2233, _, _, _, _) => {
                            h2233
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2234, _, _, _) => {
                            h2234
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2235, _, _) => {
                            h2235
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2236, _) => {
                            h2236
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2237) => {
                            h2237
                        }
                    })
                } else {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2238, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2238
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2239, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2239
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2240, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2240
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2241, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2241
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2242, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2242
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2243, _, _, _, _, _, _, _, _, _) => {
                            h2243
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2244, _, _, _, _, _, _, _, _) => {
                            h2244
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2245, _, _, _, _, _, _, _) => {
                            h2245
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2246, _, _, _, _, _, _) => {
                            h2246
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2247, _, _, _, _, _) => {
                            h2247
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2248, _, _, _, _) => {
                            h2248
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2249, _, _, _) => {
                            h2249
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2250, _, _) => {
                            h2250
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2251, _) => {
                            h2251
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2252) => {
                            h2252
                        }
                    })
                }
            } else {
                let a2253 = v1;
                let a2254 = 7u64;
                if nat_lt(a2253, a2254) {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2255, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2255
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2256, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2256
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2257, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2257
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2258, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2258
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2259, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2259
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2260, _, _, _, _, _, _, _, _, _, _) => {
                            h2260
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2261, _, _, _, _, _, _, _, _) => {
                            h2261
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2262, _, _, _, _, _, _, _) => {
                            h2262
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2263, _, _, _, _, _, _) => {
                            h2263
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2264, _, _, _, _, _) => {
                            h2264
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2265, _, _, _, _) => {
                            h2265
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2266, _, _, _) => {
                            h2266
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2267, _, _) => {
                            h2267
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2268, _) => {
                            h2268
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2269) => {
                            h2269
                        }
                    })
                } else {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2270, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2270
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2271, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2271
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2272, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2272
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2273, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2273
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2274, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2274
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2275, _, _, _, _, _, _, _, _, _, _) => {
                            h2275
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2276, _, _, _, _, _, _, _, _, _) => {
                            h2276
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2277, _, _, _, _, _, _, _) => {
                            h2277
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2278, _, _, _, _, _, _) => {
                            h2278
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2279, _, _, _, _, _) => {
                            h2279
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2280, _, _, _, _) => {
                            h2280
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2281, _, _, _) => {
                            h2281
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2282, _, _) => {
                            h2282
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2283, _) => {
                            h2283
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2284) => {
                            h2284
                        }
                    })
                }
            }
        }
    } else {
        let a2285 = v1;
        let a2286 = 12u64;
        if nat_lt(a2285, a2286) {
            let a2287 = v1;
            let a2288 = 10u64;
            if nat_lt(a2287, a2288) {
                let a2289 = v1;
                let a2290 = 9u64;
                if nat_lt(a2289, a2290) {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2291, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2291
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2292, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2292
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2293, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2293
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2294, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2294
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2295, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2295
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2296, _, _, _, _, _, _, _, _, _, _) => {
                            h2296
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2297, _, _, _, _, _, _, _, _, _) => {
                            h2297
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2298, _, _, _, _, _, _, _, _) => {
                            h2298
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2299, _, _, _, _, _, _) => {
                            h2299
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2300, _, _, _, _, _) => {
                            h2300
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2301, _, _, _, _) => {
                            h2301
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2302, _, _, _) => {
                            h2302
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2303, _, _) => {
                            h2303
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2304, _) => {
                            h2304
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2305) => {
                            h2305
                        }
                    })
                } else {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2306, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2306
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2307, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2307
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2308, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2308
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2309, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2309
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2310, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2310
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2311, _, _, _, _, _, _, _, _, _, _) => {
                            h2311
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2312, _, _, _, _, _, _, _, _, _) => {
                            h2312
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2313, _, _, _, _, _, _, _, _) => {
                            h2313
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2314, _, _, _, _, _, _, _) => {
                            h2314
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2315, _, _, _, _, _) => {
                            h2315
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2316, _, _, _, _) => {
                            h2316
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2317, _, _, _) => {
                            h2317
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2318, _, _) => {
                            h2318
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2319, _) => {
                            h2319
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2320) => {
                            h2320
                        }
                    })
                }
            } else {
                let a2321 = v1;
                let a2322 = 11u64;
                if nat_lt(a2321, a2322) {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2323, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2323
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2324, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2324
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2325, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2325
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2326, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2326
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2327, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2327
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2328, _, _, _, _, _, _, _, _, _, _) => {
                            h2328
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2329, _, _, _, _, _, _, _, _, _) => {
                            h2329
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2330, _, _, _, _, _, _, _, _) => {
                            h2330
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2331, _, _, _, _, _, _, _) => {
                            h2331
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2332, _, _, _, _, _, _) => {
                            h2332
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2333, _, _, _, _) => {
                            h2333
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2334, _, _, _) => {
                            h2334
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2335, _, _) => {
                            h2335
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2336, _) => {
                            h2336
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2337) => {
                            h2337
                        }
                    })
                } else {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2338, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2338
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2339, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2339
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2340, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2340
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2341, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2341
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2342, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2342
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2343, _, _, _, _, _, _, _, _, _, _) => {
                            h2343
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2344, _, _, _, _, _, _, _, _, _) => {
                            h2344
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2345, _, _, _, _, _, _, _, _) => {
                            h2345
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2346, _, _, _, _, _, _, _) => {
                            h2346
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2347, _, _, _, _, _, _) => {
                            h2347
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2348, _, _, _, _, _) => {
                            h2348
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2349, _, _, _) => {
                            h2349
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2350, _, _) => {
                            h2350
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2351, _) => {
                            h2351
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2352) => {
                            h2352
                        }
                    })
                }
            }
        } else {
            let a2353 = v1;
            let a2354 = 14u64;
            if nat_lt(a2353, a2354) {
                let a2355 = v1;
                let a2356 = 13u64;
                if nat_lt(a2355, a2356) {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2357, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2357
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2358, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2358
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2359, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2359
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2360, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2360
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2361, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2361
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2362, _, _, _, _, _, _, _, _, _, _) => {
                            h2362
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2363, _, _, _, _, _, _, _, _, _) => {
                            h2363
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2364, _, _, _, _, _, _, _, _) => {
                            h2364
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2365, _, _, _, _, _, _, _) => {
                            h2365
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2366, _, _, _, _, _, _) => {
                            h2366
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2367, _, _, _, _, _) => {
                            h2367
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2368, _, _, _, _) => {
                            h2368
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2369, _, _) => {
                            h2369
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2370, _) => {
                            h2370
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2371) => {
                            h2371
                        }
                    })
                } else {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2372, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2372
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2373, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2373
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2374, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2374
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2375, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2375
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2376, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2376
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2377, _, _, _, _, _, _, _, _, _, _) => {
                            h2377
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2378, _, _, _, _, _, _, _, _, _) => {
                            h2378
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2379, _, _, _, _, _, _, _, _) => {
                            h2379
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2380, _, _, _, _, _, _, _) => {
                            h2380
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2381, _, _, _, _, _, _) => {
                            h2381
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2382, _, _, _, _, _) => {
                            h2382
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2383, _, _, _, _) => {
                            h2383
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2384, _, _, _) => {
                            h2384
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2385, _) => {
                            h2385
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2386) => {
                            h2386
                        }
                    })
                }
            } else {
                let a2387 = v1;
                let a2388 = 15u64;
                if nat_lt(a2387, a2388) {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2389, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2389
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2390, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2390
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2391, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2391
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2392, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2392
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2393, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2393
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2394, _, _, _, _, _, _, _, _, _, _) => {
                            h2394
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2395, _, _, _, _, _, _, _, _, _) => {
                            h2395
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2396, _, _, _, _, _, _, _, _) => {
                            h2396
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2397, _, _, _, _, _, _, _) => {
                            h2397
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2398, _, _, _, _, _, _) => {
                            h2398
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2399, _, _, _, _, _) => {
                            h2399
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2400, _, _, _, _) => {
                            h2400
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2401, _, _, _) => {
                            h2401
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2402, _, _) => {
                            h2402
                        }
                    }, v2, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2403) => {
                            h2403
                        }
                    })
                } else {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2404, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2404
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2405, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2405
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2406, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2406
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2407, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2407
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2408, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2408
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2409, _, _, _, _, _, _, _, _, _, _) => {
                            h2409
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2410, _, _, _, _, _, _, _, _, _) => {
                            h2410
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2411, _, _, _, _, _, _, _, _) => {
                            h2411
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2412, _, _, _, _, _, _, _) => {
                            h2412
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2413, _, _, _, _, _, _) => {
                            h2413
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2414, _, _, _, _, _) => {
                            h2414
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2415, _, _, _, _) => {
                            h2415
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2416, _, _, _) => {
                            h2416
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2417, _, _) => {
                            h2417
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2418, _) => {
                            h2418
                        }
                    }, v2)
                }
            }
        }
    }
}

pub fn f123(v0: Adt18) -> R<Adt18> {
    let v1: u32 = f137(match v0.clone() {
        Adt18::C0(_, _, _, _, _, _, _, _, _, h2419) => {
            h2419
        }
    });
    let v2: u32 = f116(match v0.clone() {
        Adt18::C0(_, _, _, _, _, _, _, h2420, _, _) => {
            h2420
        }
    }, f116(f138(match v0.clone() {
        Adt18::C0(_, _, _, _, h2421, _, _, _, _, _) => {
            h2421
        }
    }), f116(f139(match v0.clone() {
        Adt18::C0(_, _, _, _, h2422, _, _, _, _, _) => {
            h2422
        }
    }, match v0.clone() {
        Adt18::C0(_, _, _, _, _, h2423, _, _, _, _) => {
            h2423
        }
    }, match v0.clone() {
        Adt18::C0(_, _, _, _, _, _, h2424, _, _, _) => {
            h2424
        }
    }), f116(v1, {
        let m2427 = match v0.clone() {
            Adt18::C0(_, _, _, _, _, _, _, _, h2426, _) => {
                h2426
            }
        };
        match m2427 {
            Adt17::C0(h2425, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                h2425
            }
        }
    }))));
    let v3: u32 = f116(f140(match v0.clone() {
        Adt18::C0(h2428, _, _, _, _, _, _, _, _, _) => {
            h2428
        }
    }), f141(match v0.clone() {
        Adt18::C0(h2429, _, _, _, _, _, _, _, _, _) => {
            h2429
        }
    }, match v0.clone() {
        Adt18::C0(_, h2430, _, _, _, _, _, _, _, _) => {
            h2430
        }
    }, match v0.clone() {
        Adt18::C0(_, _, h2431, _, _, _, _, _, _, _) => {
            h2431
        }
    }));
    Ok(Adt18::C0(f116(v2, v3), match v0.clone() {
        Adt18::C0(h2432, _, _, _, _, _, _, _, _, _) => {
            h2432
        }
    }, match v0.clone() {
        Adt18::C0(_, h2433, _, _, _, _, _, _, _, _) => {
            h2433
        }
    }, match v0.clone() {
        Adt18::C0(_, _, h2434, _, _, _, _, _, _, _) => {
            h2434
        }
    }, f116(match v0.clone() {
        Adt18::C0(_, _, _, h2435, _, _, _, _, _, _) => {
            h2435
        }
    }, v2), match v0.clone() {
        Adt18::C0(_, _, _, _, h2436, _, _, _, _, _) => {
            h2436
        }
    }, match v0.clone() {
        Adt18::C0(_, _, _, _, _, h2437, _, _, _, _) => {
            h2437
        }
    }, match v0.clone() {
        Adt18::C0(_, _, _, _, _, _, h2438, _, _, _) => {
            h2438
        }
    }, f142(match v0.clone() {
        Adt18::C0(_, _, _, _, _, _, _, _, h2439, _) => {
            h2439
        }
    }), {
        let a2441 = match v0 {
            Adt18::C0(_, _, _, _, _, _, _, _, _, h2440) => {
                h2440
            }
        };
        let a2442 = 1u64;
        nat_add(a2441, a2442)?
    }))
}

pub fn f124(v0: u32) -> u64 {
    let m2444 = {
        let a2443 = v0;
        fixed_u64::convert(i128::from(a2443))
    };
    match m2444 {
        None => {
            0u64
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f125(v0: u64) -> u32 {
    let m2448 = {
        let a2447 = {
            let a2445 = v0;
            let a2446 = 4294967295u64;
            fixed_u64::bit_and(a2445, a2446)
        };
        fixed_u32::convert(i128::from(a2447))
    };
    match m2448 {
        None => {
            0u32
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f126(v0: u64) -> u32 {
    let (_, h2449) = {
        let (_, h2450) = f30(Fn4::F143, 32u64, (v0, (1u32, 0u32)));
        h2450
    };
    h2449
}

pub fn f127(v0: u32, v1: u32, v2: u32, v3: u32) -> u32 {
    let a2455 = f144(v0, 24u32);
    let a2456 = {
        let a2453 = f144(v1, 16u32);
        let a2454 = {
            let a2451 = f144(v2, 8u32);
            let a2452 = v3;
            fixed_u32::bit_or(a2451, a2452)
        };
        fixed_u32::bit_or(a2453, a2454)
    };
    fixed_u32::bit_or(a2455, a2456)
}

pub fn f128(v0: Bytes, v1: u64, v2: u64) -> u32 {
    let a2457 = v2;
    let a2458 = v1;
    if nat_lt(a2457, a2458) {
        let m2461 = {
            let a2459 = v0;
            let a2460 = v2;
            index_bytes(a2459, a2460)
        };
        match m2461 {
            None => {
                0u32
            }
            Some(v3) => {
                let m2463 = {
                    let a2462 = v3;
                    fixed_u32::convert(i128::from(a2462))
                };
                match m2463 {
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
        let a2464 = v2;
        let a2465 = v1;
        if nat_eq(a2464, a2465) {
            128u32
        } else {
            0u32
        }
    }
}

pub fn f129(v0: u64) -> Adt19 {
    let a2466 = v0;
    let a2467 = 64u64;
    if nat_lt(a2466, a2467) {
        let a2468 = v0;
        let a2469 = 32u64;
        if nat_lt(a2468, a2469) {
            let a2470 = v0;
            let a2471 = 16u64;
            if nat_lt(a2470, a2471) {
                let a2472 = v0;
                let a2473 = 8u64;
                if nat_lt(a2472, a2473) {
                    let a2474 = v0;
                    let a2475 = 4u64;
                    if nat_lt(a2474, a2475) {
                        let a2476 = v0;
                        let a2477 = 2u64;
                        if nat_lt(a2476, a2477) {
                            let a2478 = v0;
                            let a2479 = 1u64;
                            if nat_lt(a2478, a2479) {
                                Adt19::C0(0u64, 0u64, Adt15::C0, 0u64, 4u64)
                            } else {
                                Adt19::C0(0u64, 1u64, Adt15::C1, 0u64, 0u64)
                            }
                        } else {
                            let a2480 = v0;
                            let a2481 = 3u64;
                            if nat_lt(a2480, a2481) {
                                Adt19::C0(0u64, 2u64, Adt15::C2, 12u64, 0u64)
                            } else {
                                Adt19::C0(0u64, 3u64, Adt15::C3, 12u64, 16u64)
                            }
                        }
                    } else {
                        let a2482 = v0;
                        let a2483 = 6u64;
                        if nat_lt(a2482, a2483) {
                            let a2484 = v0;
                            let a2485 = 5u64;
                            if nat_lt(a2484, a2485) {
                                Adt19::C0(0u64, 4u64, Adt15::C0, 8u64, 12u64)
                            } else {
                                Adt19::C0(0u64, 5u64, Adt15::C2, 4u64, 8u64)
                            }
                        } else {
                            let a2486 = v0;
                            let a2487 = 7u64;
                            if nat_lt(a2486, a2487) {
                                Adt19::C0(0u64, 6u64, Adt15::C3, 4u64, 12u64)
                            } else {
                                Adt19::C0(0u64, 7u64, Adt15::C0, 0u64, 4u64)
                            }
                        }
                    }
                } else {
                    let a2488 = v0;
                    let a2489 = 12u64;
                    if nat_lt(a2488, a2489) {
                        let a2490 = v0;
                        let a2491 = 10u64;
                        if nat_lt(a2490, a2491) {
                            let a2492 = v0;
                            let a2493 = 9u64;
                            if nat_lt(a2492, a2493) {
                                Adt19::C0(0u64, 8u64, Adt15::C1, 0u64, 1u64)
                            } else {
                                Adt19::C0(0u64, 9u64, Adt15::C2, 12u64, 0u64)
                            }
                        } else {
                            let a2494 = v0;
                            let a2495 = 11u64;
                            if nat_lt(a2494, a2495) {
                                Adt19::C0(0u64, 10u64, Adt15::C3, 12u64, 8u64)
                            } else {
                                Adt19::C0(0u64, 11u64, Adt15::C0, 8u64, 12u64)
                            }
                        }
                    } else {
                        let a2496 = v0;
                        let a2497 = 14u64;
                        if nat_lt(a2496, a2497) {
                            let a2498 = v0;
                            let a2499 = 13u64;
                            if nat_lt(a2498, a2499) {
                                Adt19::C0(0u64, 12u64, Adt15::C2, 4u64, 8u64)
                            } else {
                                Adt19::C0(0u64, 13u64, Adt15::C3, 4u64, 7u64)
                            }
                        } else {
                            let a2500 = v0;
                            let a2501 = 15u64;
                            if nat_lt(a2500, a2501) {
                                Adt19::C0(1u64, 0u64, Adt15::C0, 1u64, 5u64)
                            } else {
                                Adt19::C0(1u64, 1u64, Adt15::C1, 1u64, 2u64)
                            }
                        }
                    }
                }
            } else {
                let a2502 = v0;
                let a2503 = 24u64;
                if nat_lt(a2502, a2503) {
                    let a2504 = v0;
                    let a2505 = 20u64;
                    if nat_lt(a2504, a2505) {
                        let a2506 = v0;
                        let a2507 = 18u64;
                        if nat_lt(a2506, a2507) {
                            let a2508 = v0;
                            let a2509 = 17u64;
                            if nat_lt(a2508, a2509) {
                                Adt19::C0(1u64, 2u64, Adt15::C2, 13u64, 1u64)
                            } else {
                                Adt19::C0(1u64, 3u64, Adt15::C3, 13u64, 16u64)
                            }
                        } else {
                            let a2510 = v0;
                            let a2511 = 19u64;
                            if nat_lt(a2510, a2511) {
                                Adt19::C0(1u64, 4u64, Adt15::C0, 9u64, 13u64)
                            } else {
                                Adt19::C0(1u64, 5u64, Adt15::C2, 5u64, 9u64)
                            }
                        }
                    } else {
                        let a2512 = v0;
                        let a2513 = 22u64;
                        if nat_lt(a2512, a2513) {
                            let a2514 = v0;
                            let a2515 = 21u64;
                            if nat_lt(a2514, a2515) {
                                Adt19::C0(1u64, 6u64, Adt15::C3, 5u64, 12u64)
                            } else {
                                Adt19::C0(1u64, 7u64, Adt15::C0, 1u64, 5u64)
                            }
                        } else {
                            let a2516 = v0;
                            let a2517 = 23u64;
                            if nat_lt(a2516, a2517) {
                                Adt19::C0(1u64, 8u64, Adt15::C1, 1u64, 3u64)
                            } else {
                                Adt19::C0(1u64, 9u64, Adt15::C2, 13u64, 1u64)
                            }
                        }
                    }
                } else {
                    let a2518 = v0;
                    let a2519 = 28u64;
                    if nat_lt(a2518, a2519) {
                        let a2520 = v0;
                        let a2521 = 26u64;
                        if nat_lt(a2520, a2521) {
                            let a2522 = v0;
                            let a2523 = 25u64;
                            if nat_lt(a2522, a2523) {
                                Adt19::C0(1u64, 10u64, Adt15::C3, 13u64, 8u64)
                            } else {
                                Adt19::C0(1u64, 11u64, Adt15::C0, 9u64, 13u64)
                            }
                        } else {
                            let a2524 = v0;
                            let a2525 = 27u64;
                            if nat_lt(a2524, a2525) {
                                Adt19::C0(1u64, 12u64, Adt15::C2, 5u64, 9u64)
                            } else {
                                Adt19::C0(1u64, 13u64, Adt15::C3, 5u64, 7u64)
                            }
                        }
                    } else {
                        let a2526 = v0;
                        let a2527 = 30u64;
                        if nat_lt(a2526, a2527) {
                            let a2528 = v0;
                            let a2529 = 29u64;
                            if nat_lt(a2528, a2529) {
                                Adt19::C0(2u64, 0u64, Adt15::C0, 2u64, 6u64)
                            } else {
                                Adt19::C0(2u64, 1u64, Adt15::C1, 2u64, 4u64)
                            }
                        } else {
                            let a2530 = v0;
                            let a2531 = 31u64;
                            if nat_lt(a2530, a2531) {
                                Adt19::C0(2u64, 2u64, Adt15::C2, 14u64, 2u64)
                            } else {
                                Adt19::C0(2u64, 3u64, Adt15::C3, 14u64, 16u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a2532 = v0;
            let a2533 = 48u64;
            if nat_lt(a2532, a2533) {
                let a2534 = v0;
                let a2535 = 40u64;
                if nat_lt(a2534, a2535) {
                    let a2536 = v0;
                    let a2537 = 36u64;
                    if nat_lt(a2536, a2537) {
                        let a2538 = v0;
                        let a2539 = 34u64;
                        if nat_lt(a2538, a2539) {
                            let a2540 = v0;
                            let a2541 = 33u64;
                            if nat_lt(a2540, a2541) {
                                Adt19::C0(2u64, 4u64, Adt15::C0, 10u64, 14u64)
                            } else {
                                Adt19::C0(2u64, 5u64, Adt15::C2, 6u64, 10u64)
                            }
                        } else {
                            let a2542 = v0;
                            let a2543 = 35u64;
                            if nat_lt(a2542, a2543) {
                                Adt19::C0(2u64, 6u64, Adt15::C3, 6u64, 12u64)
                            } else {
                                Adt19::C0(2u64, 7u64, Adt15::C0, 2u64, 6u64)
                            }
                        }
                    } else {
                        let a2544 = v0;
                        let a2545 = 38u64;
                        if nat_lt(a2544, a2545) {
                            let a2546 = v0;
                            let a2547 = 37u64;
                            if nat_lt(a2546, a2547) {
                                Adt19::C0(2u64, 8u64, Adt15::C1, 2u64, 5u64)
                            } else {
                                Adt19::C0(2u64, 9u64, Adt15::C2, 14u64, 2u64)
                            }
                        } else {
                            let a2548 = v0;
                            let a2549 = 39u64;
                            if nat_lt(a2548, a2549) {
                                Adt19::C0(2u64, 10u64, Adt15::C3, 14u64, 8u64)
                            } else {
                                Adt19::C0(2u64, 11u64, Adt15::C0, 10u64, 14u64)
                            }
                        }
                    }
                } else {
                    let a2550 = v0;
                    let a2551 = 44u64;
                    if nat_lt(a2550, a2551) {
                        let a2552 = v0;
                        let a2553 = 42u64;
                        if nat_lt(a2552, a2553) {
                            let a2554 = v0;
                            let a2555 = 41u64;
                            if nat_lt(a2554, a2555) {
                                Adt19::C0(2u64, 12u64, Adt15::C2, 6u64, 10u64)
                            } else {
                                Adt19::C0(2u64, 13u64, Adt15::C3, 6u64, 7u64)
                            }
                        } else {
                            let a2556 = v0;
                            let a2557 = 43u64;
                            if nat_lt(a2556, a2557) {
                                Adt19::C0(3u64, 0u64, Adt15::C0, 3u64, 7u64)
                            } else {
                                Adt19::C0(3u64, 1u64, Adt15::C1, 3u64, 6u64)
                            }
                        }
                    } else {
                        let a2558 = v0;
                        let a2559 = 46u64;
                        if nat_lt(a2558, a2559) {
                            let a2560 = v0;
                            let a2561 = 45u64;
                            if nat_lt(a2560, a2561) {
                                Adt19::C0(3u64, 2u64, Adt15::C2, 15u64, 3u64)
                            } else {
                                Adt19::C0(3u64, 3u64, Adt15::C3, 15u64, 16u64)
                            }
                        } else {
                            let a2562 = v0;
                            let a2563 = 47u64;
                            if nat_lt(a2562, a2563) {
                                Adt19::C0(3u64, 4u64, Adt15::C0, 11u64, 15u64)
                            } else {
                                Adt19::C0(3u64, 5u64, Adt15::C2, 7u64, 11u64)
                            }
                        }
                    }
                }
            } else {
                let a2564 = v0;
                let a2565 = 56u64;
                if nat_lt(a2564, a2565) {
                    let a2566 = v0;
                    let a2567 = 52u64;
                    if nat_lt(a2566, a2567) {
                        let a2568 = v0;
                        let a2569 = 50u64;
                        if nat_lt(a2568, a2569) {
                            let a2570 = v0;
                            let a2571 = 49u64;
                            if nat_lt(a2570, a2571) {
                                Adt19::C0(3u64, 6u64, Adt15::C3, 7u64, 12u64)
                            } else {
                                Adt19::C0(3u64, 7u64, Adt15::C0, 3u64, 7u64)
                            }
                        } else {
                            let a2572 = v0;
                            let a2573 = 51u64;
                            if nat_lt(a2572, a2573) {
                                Adt19::C0(3u64, 8u64, Adt15::C1, 3u64, 7u64)
                            } else {
                                Adt19::C0(3u64, 9u64, Adt15::C2, 15u64, 3u64)
                            }
                        }
                    } else {
                        let a2574 = v0;
                        let a2575 = 54u64;
                        if nat_lt(a2574, a2575) {
                            let a2576 = v0;
                            let a2577 = 53u64;
                            if nat_lt(a2576, a2577) {
                                Adt19::C0(3u64, 10u64, Adt15::C3, 15u64, 8u64)
                            } else {
                                Adt19::C0(3u64, 11u64, Adt15::C0, 11u64, 15u64)
                            }
                        } else {
                            let a2578 = v0;
                            let a2579 = 55u64;
                            if nat_lt(a2578, a2579) {
                                Adt19::C0(3u64, 12u64, Adt15::C2, 7u64, 11u64)
                            } else {
                                Adt19::C0(3u64, 13u64, Adt15::C3, 7u64, 7u64)
                            }
                        }
                    }
                } else {
                    let a2580 = v0;
                    let a2581 = 60u64;
                    if nat_lt(a2580, a2581) {
                        let a2582 = v0;
                        let a2583 = 58u64;
                        if nat_lt(a2582, a2583) {
                            let a2584 = v0;
                            let a2585 = 57u64;
                            if nat_lt(a2584, a2585) {
                                Adt19::C0(4u64, 0u64, Adt15::C0, 0u64, 5u64)
                            } else {
                                Adt19::C0(4u64, 1u64, Adt15::C1, 0u64, 8u64)
                            }
                        } else {
                            let a2586 = v0;
                            let a2587 = 59u64;
                            if nat_lt(a2586, a2587) {
                                Adt19::C0(4u64, 2u64, Adt15::C2, 15u64, 0u64)
                            } else {
                                Adt19::C0(4u64, 3u64, Adt15::C3, 15u64, 16u64)
                            }
                        }
                    } else {
                        let a2588 = v0;
                        let a2589 = 62u64;
                        if nat_lt(a2588, a2589) {
                            let a2590 = v0;
                            let a2591 = 61u64;
                            if nat_lt(a2590, a2591) {
                                Adt19::C0(4u64, 4u64, Adt15::C0, 10u64, 15u64)
                            } else {
                                Adt19::C0(4u64, 5u64, Adt15::C2, 5u64, 10u64)
                            }
                        } else {
                            let a2592 = v0;
                            let a2593 = 63u64;
                            if nat_lt(a2592, a2593) {
                                Adt19::C0(4u64, 6u64, Adt15::C3, 5u64, 12u64)
                            } else {
                                Adt19::C0(4u64, 7u64, Adt15::C0, 0u64, 5u64)
                            }
                        }
                    }
                }
            }
        }
    } else {
        let a2594 = v0;
        let a2595 = 96u64;
        if nat_lt(a2594, a2595) {
            let a2596 = v0;
            let a2597 = 80u64;
            if nat_lt(a2596, a2597) {
                let a2598 = v0;
                let a2599 = 72u64;
                if nat_lt(a2598, a2599) {
                    let a2600 = v0;
                    let a2601 = 68u64;
                    if nat_lt(a2600, a2601) {
                        let a2602 = v0;
                        let a2603 = 66u64;
                        if nat_lt(a2602, a2603) {
                            let a2604 = v0;
                            let a2605 = 65u64;
                            if nat_lt(a2604, a2605) {
                                Adt19::C0(4u64, 8u64, Adt15::C1, 0u64, 9u64)
                            } else {
                                Adt19::C0(4u64, 9u64, Adt15::C2, 15u64, 0u64)
                            }
                        } else {
                            let a2606 = v0;
                            let a2607 = 67u64;
                            if nat_lt(a2606, a2607) {
                                Adt19::C0(4u64, 10u64, Adt15::C3, 15u64, 8u64)
                            } else {
                                Adt19::C0(4u64, 11u64, Adt15::C0, 10u64, 15u64)
                            }
                        }
                    } else {
                        let a2608 = v0;
                        let a2609 = 70u64;
                        if nat_lt(a2608, a2609) {
                            let a2610 = v0;
                            let a2611 = 69u64;
                            if nat_lt(a2610, a2611) {
                                Adt19::C0(4u64, 12u64, Adt15::C2, 5u64, 10u64)
                            } else {
                                Adt19::C0(4u64, 13u64, Adt15::C3, 5u64, 7u64)
                            }
                        } else {
                            let a2612 = v0;
                            let a2613 = 71u64;
                            if nat_lt(a2612, a2613) {
                                Adt19::C0(5u64, 0u64, Adt15::C0, 1u64, 6u64)
                            } else {
                                Adt19::C0(5u64, 1u64, Adt15::C1, 1u64, 10u64)
                            }
                        }
                    }
                } else {
                    let a2614 = v0;
                    let a2615 = 76u64;
                    if nat_lt(a2614, a2615) {
                        let a2616 = v0;
                        let a2617 = 74u64;
                        if nat_lt(a2616, a2617) {
                            let a2618 = v0;
                            let a2619 = 73u64;
                            if nat_lt(a2618, a2619) {
                                Adt19::C0(5u64, 2u64, Adt15::C2, 12u64, 1u64)
                            } else {
                                Adt19::C0(5u64, 3u64, Adt15::C3, 12u64, 16u64)
                            }
                        } else {
                            let a2620 = v0;
                            let a2621 = 75u64;
                            if nat_lt(a2620, a2621) {
                                Adt19::C0(5u64, 4u64, Adt15::C0, 11u64, 12u64)
                            } else {
                                Adt19::C0(5u64, 5u64, Adt15::C2, 6u64, 11u64)
                            }
                        }
                    } else {
                        let a2622 = v0;
                        let a2623 = 78u64;
                        if nat_lt(a2622, a2623) {
                            let a2624 = v0;
                            let a2625 = 77u64;
                            if nat_lt(a2624, a2625) {
                                Adt19::C0(5u64, 6u64, Adt15::C3, 6u64, 12u64)
                            } else {
                                Adt19::C0(5u64, 7u64, Adt15::C0, 1u64, 6u64)
                            }
                        } else {
                            let a2626 = v0;
                            let a2627 = 79u64;
                            if nat_lt(a2626, a2627) {
                                Adt19::C0(5u64, 8u64, Adt15::C1, 1u64, 11u64)
                            } else {
                                Adt19::C0(5u64, 9u64, Adt15::C2, 12u64, 1u64)
                            }
                        }
                    }
                }
            } else {
                let a2628 = v0;
                let a2629 = 88u64;
                if nat_lt(a2628, a2629) {
                    let a2630 = v0;
                    let a2631 = 84u64;
                    if nat_lt(a2630, a2631) {
                        let a2632 = v0;
                        let a2633 = 82u64;
                        if nat_lt(a2632, a2633) {
                            let a2634 = v0;
                            let a2635 = 81u64;
                            if nat_lt(a2634, a2635) {
                                Adt19::C0(5u64, 10u64, Adt15::C3, 12u64, 8u64)
                            } else {
                                Adt19::C0(5u64, 11u64, Adt15::C0, 11u64, 12u64)
                            }
                        } else {
                            let a2636 = v0;
                            let a2637 = 83u64;
                            if nat_lt(a2636, a2637) {
                                Adt19::C0(5u64, 12u64, Adt15::C2, 6u64, 11u64)
                            } else {
                                Adt19::C0(5u64, 13u64, Adt15::C3, 6u64, 7u64)
                            }
                        }
                    } else {
                        let a2638 = v0;
                        let a2639 = 86u64;
                        if nat_lt(a2638, a2639) {
                            let a2640 = v0;
                            let a2641 = 85u64;
                            if nat_lt(a2640, a2641) {
                                Adt19::C0(6u64, 0u64, Adt15::C0, 2u64, 7u64)
                            } else {
                                Adt19::C0(6u64, 1u64, Adt15::C1, 2u64, 12u64)
                            }
                        } else {
                            let a2642 = v0;
                            let a2643 = 87u64;
                            if nat_lt(a2642, a2643) {
                                Adt19::C0(6u64, 2u64, Adt15::C2, 13u64, 2u64)
                            } else {
                                Adt19::C0(6u64, 3u64, Adt15::C3, 13u64, 16u64)
                            }
                        }
                    }
                } else {
                    let a2644 = v0;
                    let a2645 = 92u64;
                    if nat_lt(a2644, a2645) {
                        let a2646 = v0;
                        let a2647 = 90u64;
                        if nat_lt(a2646, a2647) {
                            let a2648 = v0;
                            let a2649 = 89u64;
                            if nat_lt(a2648, a2649) {
                                Adt19::C0(6u64, 4u64, Adt15::C0, 8u64, 13u64)
                            } else {
                                Adt19::C0(6u64, 5u64, Adt15::C2, 7u64, 8u64)
                            }
                        } else {
                            let a2650 = v0;
                            let a2651 = 91u64;
                            if nat_lt(a2650, a2651) {
                                Adt19::C0(6u64, 6u64, Adt15::C3, 7u64, 12u64)
                            } else {
                                Adt19::C0(6u64, 7u64, Adt15::C0, 2u64, 7u64)
                            }
                        }
                    } else {
                        let a2652 = v0;
                        let a2653 = 94u64;
                        if nat_lt(a2652, a2653) {
                            let a2654 = v0;
                            let a2655 = 93u64;
                            if nat_lt(a2654, a2655) {
                                Adt19::C0(6u64, 8u64, Adt15::C1, 2u64, 13u64)
                            } else {
                                Adt19::C0(6u64, 9u64, Adt15::C2, 13u64, 2u64)
                            }
                        } else {
                            let a2656 = v0;
                            let a2657 = 95u64;
                            if nat_lt(a2656, a2657) {
                                Adt19::C0(6u64, 10u64, Adt15::C3, 13u64, 8u64)
                            } else {
                                Adt19::C0(6u64, 11u64, Adt15::C0, 8u64, 13u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a2658 = v0;
            let a2659 = 112u64;
            if nat_lt(a2658, a2659) {
                let a2660 = v0;
                let a2661 = 104u64;
                if nat_lt(a2660, a2661) {
                    let a2662 = v0;
                    let a2663 = 100u64;
                    if nat_lt(a2662, a2663) {
                        let a2664 = v0;
                        let a2665 = 98u64;
                        if nat_lt(a2664, a2665) {
                            let a2666 = v0;
                            let a2667 = 97u64;
                            if nat_lt(a2666, a2667) {
                                Adt19::C0(6u64, 12u64, Adt15::C2, 7u64, 8u64)
                            } else {
                                Adt19::C0(6u64, 13u64, Adt15::C3, 7u64, 7u64)
                            }
                        } else {
                            let a2668 = v0;
                            let a2669 = 99u64;
                            if nat_lt(a2668, a2669) {
                                Adt19::C0(7u64, 0u64, Adt15::C0, 3u64, 4u64)
                            } else {
                                Adt19::C0(7u64, 1u64, Adt15::C1, 3u64, 14u64)
                            }
                        }
                    } else {
                        let a2670 = v0;
                        let a2671 = 102u64;
                        if nat_lt(a2670, a2671) {
                            let a2672 = v0;
                            let a2673 = 101u64;
                            if nat_lt(a2672, a2673) {
                                Adt19::C0(7u64, 2u64, Adt15::C2, 14u64, 3u64)
                            } else {
                                Adt19::C0(7u64, 3u64, Adt15::C3, 14u64, 16u64)
                            }
                        } else {
                            let a2674 = v0;
                            let a2675 = 103u64;
                            if nat_lt(a2674, a2675) {
                                Adt19::C0(7u64, 4u64, Adt15::C0, 9u64, 14u64)
                            } else {
                                Adt19::C0(7u64, 5u64, Adt15::C2, 4u64, 9u64)
                            }
                        }
                    }
                } else {
                    let a2676 = v0;
                    let a2677 = 108u64;
                    if nat_lt(a2676, a2677) {
                        let a2678 = v0;
                        let a2679 = 106u64;
                        if nat_lt(a2678, a2679) {
                            let a2680 = v0;
                            let a2681 = 105u64;
                            if nat_lt(a2680, a2681) {
                                Adt19::C0(7u64, 6u64, Adt15::C3, 4u64, 12u64)
                            } else {
                                Adt19::C0(7u64, 7u64, Adt15::C0, 3u64, 4u64)
                            }
                        } else {
                            let a2682 = v0;
                            let a2683 = 107u64;
                            if nat_lt(a2682, a2683) {
                                Adt19::C0(7u64, 8u64, Adt15::C1, 3u64, 15u64)
                            } else {
                                Adt19::C0(7u64, 9u64, Adt15::C2, 14u64, 3u64)
                            }
                        }
                    } else {
                        let a2684 = v0;
                        let a2685 = 110u64;
                        if nat_lt(a2684, a2685) {
                            let a2686 = v0;
                            let a2687 = 109u64;
                            if nat_lt(a2686, a2687) {
                                Adt19::C0(7u64, 10u64, Adt15::C3, 14u64, 8u64)
                            } else {
                                Adt19::C0(7u64, 11u64, Adt15::C0, 9u64, 14u64)
                            }
                        } else {
                            let a2688 = v0;
                            let a2689 = 111u64;
                            if nat_lt(a2688, a2689) {
                                Adt19::C0(7u64, 12u64, Adt15::C2, 4u64, 9u64)
                            } else {
                                Adt19::C0(7u64, 13u64, Adt15::C3, 4u64, 7u64)
                            }
                        }
                    }
                }
            } else {
                let a2690 = v0;
                let a2691 = 120u64;
                if nat_lt(a2690, a2691) {
                    let a2692 = v0;
                    let a2693 = 116u64;
                    if nat_lt(a2692, a2693) {
                        let a2694 = v0;
                        let a2695 = 114u64;
                        if nat_lt(a2694, a2695) {
                            let a2696 = v0;
                            let a2697 = 113u64;
                            if nat_lt(a2696, a2697) {
                                Adt19::C0(0u64, 0u64, Adt15::C2, 0u64, 8u64)
                            } else {
                                Adt19::C0(0u64, 1u64, Adt15::C2, 1u64, 9u64)
                            }
                        } else {
                            let a2698 = v0;
                            let a2699 = 115u64;
                            if nat_lt(a2698, a2699) {
                                Adt19::C0(0u64, 2u64, Adt15::C2, 2u64, 10u64)
                            } else {
                                Adt19::C0(0u64, 3u64, Adt15::C2, 3u64, 11u64)
                            }
                        }
                    } else {
                        let a2700 = v0;
                        let a2701 = 118u64;
                        if nat_lt(a2700, a2701) {
                            let a2702 = v0;
                            let a2703 = 117u64;
                            if nat_lt(a2702, a2703) {
                                Adt19::C0(0u64, 4u64, Adt15::C2, 4u64, 12u64)
                            } else {
                                Adt19::C0(0u64, 5u64, Adt15::C2, 5u64, 13u64)
                            }
                        } else {
                            let a2704 = v0;
                            let a2705 = 119u64;
                            if nat_lt(a2704, a2705) {
                                Adt19::C0(0u64, 6u64, Adt15::C2, 6u64, 14u64)
                            } else {
                                Adt19::C0(0u64, 7u64, Adt15::C2, 7u64, 15u64)
                            }
                        }
                    }
                } else {
                    let a2706 = v0;
                    let a2707 = 124u64;
                    if nat_lt(a2706, a2707) {
                        let a2708 = v0;
                        let a2709 = 122u64;
                        if nat_lt(a2708, a2709) {
                            let a2710 = v0;
                            let a2711 = 121u64;
                            if nat_lt(a2710, a2711) {
                                Adt19::C0(0u64, 8u64, Adt15::C4, 8u64, 0u64)
                            } else {
                                Adt19::C0(0u64, 9u64, Adt15::C4, 9u64, 1u64)
                            }
                        } else {
                            let a2712 = v0;
                            let a2713 = 123u64;
                            if nat_lt(a2712, a2713) {
                                Adt19::C0(0u64, 10u64, Adt15::C4, 10u64, 2u64)
                            } else {
                                Adt19::C0(0u64, 11u64, Adt15::C4, 11u64, 3u64)
                            }
                        }
                    } else {
                        let a2714 = v0;
                        let a2715 = 126u64;
                        if nat_lt(a2714, a2715) {
                            let a2716 = v0;
                            let a2717 = 125u64;
                            if nat_lt(a2716, a2717) {
                                Adt19::C0(0u64, 12u64, Adt15::C4, 12u64, 4u64)
                            } else {
                                Adt19::C0(0u64, 13u64, Adt15::C4, 13u64, 5u64)
                            }
                        } else {
                            let a2718 = v0;
                            let a2719 = 127u64;
                            if nat_lt(a2718, a2719) {
                                Adt19::C0(0u64, 14u64, Adt15::C4, 14u64, 6u64)
                            } else {
                                Adt19::C0(0u64, 15u64, Adt15::C4, 15u64, 7u64)
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn f130(v0: Adt6, v1: u64) -> u32 {
    let a2720 = v1;
    let a2721 = 8u64;
    if nat_lt(a2720, a2721) {
        let a2722 = v1;
        let a2723 = 4u64;
        if nat_lt(a2722, a2723) {
            let a2724 = v1;
            let a2725 = 2u64;
            if nat_lt(a2724, a2725) {
                let a2726 = v1;
                let a2727 = 1u64;
                if nat_lt(a2726, a2727) {
                    match v0 {
                        Adt6::C0(h2728, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2728
                        }
                    }
                } else {
                    match v0 {
                        Adt6::C0(_, h2729, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2729
                        }
                    }
                }
            } else {
                let a2730 = v1;
                let a2731 = 3u64;
                if nat_lt(a2730, a2731) {
                    match v0 {
                        Adt6::C0(_, _, h2732, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2732
                        }
                    }
                } else {
                    match v0 {
                        Adt6::C0(_, _, _, h2733, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2733
                        }
                    }
                }
            }
        } else {
            let a2734 = v1;
            let a2735 = 6u64;
            if nat_lt(a2734, a2735) {
                let a2736 = v1;
                let a2737 = 5u64;
                if nat_lt(a2736, a2737) {
                    match v0 {
                        Adt6::C0(_, _, _, _, h2738, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2738
                        }
                    }
                } else {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, h2739, _, _, _, _, _, _, _, _, _, _) => {
                            h2739
                        }
                    }
                }
            } else {
                let a2740 = v1;
                let a2741 = 7u64;
                if nat_lt(a2740, a2741) {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, h2742, _, _, _, _, _, _, _, _, _) => {
                            h2742
                        }
                    }
                } else {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, h2743, _, _, _, _, _, _, _, _) => {
                            h2743
                        }
                    }
                }
            }
        }
    } else {
        let a2744 = v1;
        let a2745 = 12u64;
        if nat_lt(a2744, a2745) {
            let a2746 = v1;
            let a2747 = 10u64;
            if nat_lt(a2746, a2747) {
                let a2748 = v1;
                let a2749 = 9u64;
                if nat_lt(a2748, a2749) {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2750, _, _, _, _, _, _, _) => {
                            h2750
                        }
                    }
                } else {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2751, _, _, _, _, _, _) => {
                            h2751
                        }
                    }
                }
            } else {
                let a2752 = v1;
                let a2753 = 11u64;
                if nat_lt(a2752, a2753) {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2754, _, _, _, _, _) => {
                            h2754
                        }
                    }
                } else {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2755, _, _, _, _) => {
                            h2755
                        }
                    }
                }
            }
        } else {
            let a2756 = v1;
            let a2757 = 14u64;
            if nat_lt(a2756, a2757) {
                let a2758 = v1;
                let a2759 = 13u64;
                if nat_lt(a2758, a2759) {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2760, _, _, _) => {
                            h2760
                        }
                    }
                } else {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2761, _, _) => {
                            h2761
                        }
                    }
                }
            } else {
                let a2762 = v1;
                let a2763 = 15u64;
                if nat_lt(a2762, a2763) {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2764, _) => {
                            h2764
                        }
                    }
                } else {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2765) => {
                            h2765
                        }
                    }
                }
            }
        }
    }
}

pub fn f131(v0: u64, v1: u64) -> R<u64> {
    let v2: u64 = {
        let a2768 = {
            let a2766 = v0;
            let a2767 = 16u64;
            nat_mul(a2766, a2767)?
        };
        let a2769 = v1;
        nat_add(a2768, a2769)?
    };
    let a2770 = v2;
    let a2771 = 56u64;
    if nat_lt(a2770, a2771) {
        let a2772 = v2;
        let a2773 = 28u64;
        if nat_lt(a2772, a2773) {
            let a2774 = v2;
            let a2775 = 14u64;
            if nat_lt(a2774, a2775) {
                let a2776 = v2;
                let a2777 = 7u64;
                if nat_lt(a2776, a2777) {
                    let a2778 = v2;
                    let a2779 = 3u64;
                    if nat_lt(a2778, a2779) {
                        let a2780 = v2;
                        let a2781 = 1u64;
                        if nat_lt(a2780, a2781) {
                            Ok(0u64)
                        } else {
                            let a2782 = v2;
                            let a2783 = 2u64;
                            if nat_lt(a2782, a2783) {
                                Ok(1u64)
                            } else {
                                Ok(2u64)
                            }
                        }
                    } else {
                        let a2784 = v2;
                        let a2785 = 5u64;
                        if nat_lt(a2784, a2785) {
                            let a2786 = v2;
                            let a2787 = 4u64;
                            if nat_lt(a2786, a2787) {
                                Ok(3u64)
                            } else {
                                Ok(4u64)
                            }
                        } else {
                            let a2788 = v2;
                            let a2789 = 6u64;
                            if nat_lt(a2788, a2789) {
                                Ok(5u64)
                            } else {
                                Ok(6u64)
                            }
                        }
                    }
                } else {
                    let a2790 = v2;
                    let a2791 = 10u64;
                    if nat_lt(a2790, a2791) {
                        let a2792 = v2;
                        let a2793 = 8u64;
                        if nat_lt(a2792, a2793) {
                            Ok(7u64)
                        } else {
                            let a2794 = v2;
                            let a2795 = 9u64;
                            if nat_lt(a2794, a2795) {
                                Ok(8u64)
                            } else {
                                Ok(9u64)
                            }
                        }
                    } else {
                        let a2796 = v2;
                        let a2797 = 12u64;
                        if nat_lt(a2796, a2797) {
                            let a2798 = v2;
                            let a2799 = 11u64;
                            if nat_lt(a2798, a2799) {
                                Ok(10u64)
                            } else {
                                Ok(11u64)
                            }
                        } else {
                            let a2800 = v2;
                            let a2801 = 13u64;
                            if nat_lt(a2800, a2801) {
                                Ok(12u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    }
                }
            } else {
                let a2802 = v2;
                let a2803 = 21u64;
                if nat_lt(a2802, a2803) {
                    let a2804 = v2;
                    let a2805 = 17u64;
                    if nat_lt(a2804, a2805) {
                        let a2806 = v2;
                        let a2807 = 15u64;
                        if nat_lt(a2806, a2807) {
                            Ok(14u64)
                        } else {
                            let a2808 = v2;
                            let a2809 = 16u64;
                            if nat_lt(a2808, a2809) {
                                Ok(15u64)
                            } else {
                                Ok(2u64)
                            }
                        }
                    } else {
                        let a2810 = v2;
                        let a2811 = 19u64;
                        if nat_lt(a2810, a2811) {
                            let a2812 = v2;
                            let a2813 = 18u64;
                            if nat_lt(a2812, a2813) {
                                Ok(6u64)
                            } else {
                                Ok(3u64)
                            }
                        } else {
                            let a2814 = v2;
                            let a2815 = 20u64;
                            if nat_lt(a2814, a2815) {
                                Ok(10u64)
                            } else {
                                Ok(7u64)
                            }
                        }
                    }
                } else {
                    let a2816 = v2;
                    let a2817 = 24u64;
                    if nat_lt(a2816, a2817) {
                        let a2818 = v2;
                        let a2819 = 22u64;
                        if nat_lt(a2818, a2819) {
                            Ok(0u64)
                        } else {
                            let a2820 = v2;
                            let a2821 = 23u64;
                            if nat_lt(a2820, a2821) {
                                Ok(4u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    } else {
                        let a2822 = v2;
                        let a2823 = 26u64;
                        if nat_lt(a2822, a2823) {
                            let a2824 = v2;
                            let a2825 = 25u64;
                            if nat_lt(a2824, a2825) {
                                Ok(1u64)
                            } else {
                                Ok(11u64)
                            }
                        } else {
                            let a2826 = v2;
                            let a2827 = 27u64;
                            if nat_lt(a2826, a2827) {
                                Ok(12u64)
                            } else {
                                Ok(5u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a2828 = v2;
            let a2829 = 42u64;
            if nat_lt(a2828, a2829) {
                let a2830 = v2;
                let a2831 = 35u64;
                if nat_lt(a2830, a2831) {
                    let a2832 = v2;
                    let a2833 = 31u64;
                    if nat_lt(a2832, a2833) {
                        let a2834 = v2;
                        let a2835 = 29u64;
                        if nat_lt(a2834, a2835) {
                            Ok(9u64)
                        } else {
                            let a2836 = v2;
                            let a2837 = 30u64;
                            if nat_lt(a2836, a2837) {
                                Ok(14u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    } else {
                        let a2838 = v2;
                        let a2839 = 33u64;
                        if nat_lt(a2838, a2839) {
                            let a2840 = v2;
                            let a2841 = 32u64;
                            if nat_lt(a2840, a2841) {
                                Ok(8u64)
                            } else {
                                Ok(3u64)
                            }
                        } else {
                            let a2842 = v2;
                            let a2843 = 34u64;
                            if nat_lt(a2842, a2843) {
                                Ok(4u64)
                            } else {
                                Ok(10u64)
                            }
                        }
                    }
                } else {
                    let a2844 = v2;
                    let a2845 = 38u64;
                    if nat_lt(a2844, a2845) {
                        let a2846 = v2;
                        let a2847 = 36u64;
                        if nat_lt(a2846, a2847) {
                            Ok(12u64)
                        } else {
                            let a2848 = v2;
                            let a2849 = 37u64;
                            if nat_lt(a2848, a2849) {
                                Ok(13u64)
                            } else {
                                Ok(2u64)
                            }
                        }
                    } else {
                        let a2850 = v2;
                        let a2851 = 40u64;
                        if nat_lt(a2850, a2851) {
                            let a2852 = v2;
                            let a2853 = 39u64;
                            if nat_lt(a2852, a2853) {
                                Ok(7u64)
                            } else {
                                Ok(14u64)
                            }
                        } else {
                            let a2854 = v2;
                            let a2855 = 41u64;
                            if nat_lt(a2854, a2855) {
                                Ok(6u64)
                            } else {
                                Ok(5u64)
                            }
                        }
                    }
                }
            } else {
                let a2856 = v2;
                let a2857 = 49u64;
                if nat_lt(a2856, a2857) {
                    let a2858 = v2;
                    let a2859 = 45u64;
                    if nat_lt(a2858, a2859) {
                        let a2860 = v2;
                        let a2861 = 43u64;
                        if nat_lt(a2860, a2861) {
                            Ok(9u64)
                        } else {
                            let a2862 = v2;
                            let a2863 = 44u64;
                            if nat_lt(a2862, a2863) {
                                Ok(0u64)
                            } else {
                                Ok(11u64)
                            }
                        }
                    } else {
                        let a2864 = v2;
                        let a2865 = 47u64;
                        if nat_lt(a2864, a2865) {
                            let a2866 = v2;
                            let a2867 = 46u64;
                            if nat_lt(a2866, a2867) {
                                Ok(15u64)
                            } else {
                                Ok(8u64)
                            }
                        } else {
                            let a2868 = v2;
                            let a2869 = 48u64;
                            if nat_lt(a2868, a2869) {
                                Ok(1u64)
                            } else {
                                Ok(10u64)
                            }
                        }
                    }
                } else {
                    let a2870 = v2;
                    let a2871 = 52u64;
                    if nat_lt(a2870, a2871) {
                        let a2872 = v2;
                        let a2873 = 50u64;
                        if nat_lt(a2872, a2873) {
                            Ok(7u64)
                        } else {
                            let a2874 = v2;
                            let a2875 = 51u64;
                            if nat_lt(a2874, a2875) {
                                Ok(12u64)
                            } else {
                                Ok(9u64)
                            }
                        }
                    } else {
                        let a2876 = v2;
                        let a2877 = 54u64;
                        if nat_lt(a2876, a2877) {
                            let a2878 = v2;
                            let a2879 = 53u64;
                            if nat_lt(a2878, a2879) {
                                Ok(14u64)
                            } else {
                                Ok(3u64)
                            }
                        } else {
                            let a2880 = v2;
                            let a2881 = 55u64;
                            if nat_lt(a2880, a2881) {
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
        let a2882 = v2;
        let a2883 = 84u64;
        if nat_lt(a2882, a2883) {
            let a2884 = v2;
            let a2885 = 70u64;
            if nat_lt(a2884, a2885) {
                let a2886 = v2;
                let a2887 = 63u64;
                if nat_lt(a2886, a2887) {
                    let a2888 = v2;
                    let a2889 = 59u64;
                    if nat_lt(a2888, a2889) {
                        let a2890 = v2;
                        let a2891 = 57u64;
                        if nat_lt(a2890, a2891) {
                            Ok(4u64)
                        } else {
                            let a2892 = v2;
                            let a2893 = 58u64;
                            if nat_lt(a2892, a2893) {
                                Ok(0u64)
                            } else {
                                Ok(11u64)
                            }
                        }
                    } else {
                        let a2894 = v2;
                        let a2895 = 61u64;
                        if nat_lt(a2894, a2895) {
                            let a2896 = v2;
                            let a2897 = 60u64;
                            if nat_lt(a2896, a2897) {
                                Ok(2u64)
                            } else {
                                Ok(5u64)
                            }
                        } else {
                            let a2898 = v2;
                            let a2899 = 62u64;
                            if nat_lt(a2898, a2899) {
                                Ok(8u64)
                            } else {
                                Ok(1u64)
                            }
                        }
                    }
                } else {
                    let a2900 = v2;
                    let a2901 = 66u64;
                    if nat_lt(a2900, a2901) {
                        let a2902 = v2;
                        let a2903 = 64u64;
                        if nat_lt(a2902, a2903) {
                            Ok(6u64)
                        } else {
                            let a2904 = v2;
                            let a2905 = 65u64;
                            if nat_lt(a2904, a2905) {
                                Ok(12u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    } else {
                        let a2906 = v2;
                        let a2907 = 68u64;
                        if nat_lt(a2906, a2907) {
                            let a2908 = v2;
                            let a2909 = 67u64;
                            if nat_lt(a2908, a2909) {
                                Ok(9u64)
                            } else {
                                Ok(11u64)
                            }
                        } else {
                            let a2910 = v2;
                            let a2911 = 69u64;
                            if nat_lt(a2910, a2911) {
                                Ok(15u64)
                            } else {
                                Ok(10u64)
                            }
                        }
                    }
                }
            } else {
                let a2912 = v2;
                let a2913 = 77u64;
                if nat_lt(a2912, a2913) {
                    let a2914 = v2;
                    let a2915 = 73u64;
                    if nat_lt(a2914, a2915) {
                        let a2916 = v2;
                        let a2917 = 71u64;
                        if nat_lt(a2916, a2917) {
                            Ok(14u64)
                        } else {
                            let a2918 = v2;
                            let a2919 = 72u64;
                            if nat_lt(a2918, a2919) {
                                Ok(8u64)
                            } else {
                                Ok(7u64)
                            }
                        }
                    } else {
                        let a2920 = v2;
                        let a2921 = 75u64;
                        if nat_lt(a2920, a2921) {
                            let a2922 = v2;
                            let a2923 = 74u64;
                            if nat_lt(a2922, a2923) {
                                Ok(2u64)
                            } else {
                                Ok(5u64)
                            }
                        } else {
                            let a2924 = v2;
                            let a2925 = 76u64;
                            if nat_lt(a2924, a2925) {
                                Ok(3u64)
                            } else {
                                Ok(0u64)
                            }
                        }
                    }
                } else {
                    let a2926 = v2;
                    let a2927 = 80u64;
                    if nat_lt(a2926, a2927) {
                        let a2928 = v2;
                        let a2929 = 78u64;
                        if nat_lt(a2928, a2929) {
                            Ok(1u64)
                        } else {
                            let a2930 = v2;
                            let a2931 = 79u64;
                            if nat_lt(a2930, a2931) {
                                Ok(6u64)
                            } else {
                                Ok(4u64)
                            }
                        }
                    } else {
                        let a2932 = v2;
                        let a2933 = 82u64;
                        if nat_lt(a2932, a2933) {
                            let a2934 = v2;
                            let a2935 = 81u64;
                            if nat_lt(a2934, a2935) {
                                Ok(9u64)
                            } else {
                                Ok(14u64)
                            }
                        } else {
                            let a2936 = v2;
                            let a2937 = 83u64;
                            if nat_lt(a2936, a2937) {
                                Ok(11u64)
                            } else {
                                Ok(5u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a2938 = v2;
            let a2939 = 98u64;
            if nat_lt(a2938, a2939) {
                let a2940 = v2;
                let a2941 = 91u64;
                if nat_lt(a2940, a2941) {
                    let a2942 = v2;
                    let a2943 = 87u64;
                    if nat_lt(a2942, a2943) {
                        let a2944 = v2;
                        let a2945 = 85u64;
                        if nat_lt(a2944, a2945) {
                            Ok(8u64)
                        } else {
                            let a2946 = v2;
                            let a2947 = 86u64;
                            if nat_lt(a2946, a2947) {
                                Ok(12u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    } else {
                        let a2948 = v2;
                        let a2949 = 89u64;
                        if nat_lt(a2948, a2949) {
                            let a2950 = v2;
                            let a2951 = 88u64;
                            if nat_lt(a2950, a2951) {
                                Ok(1u64)
                            } else {
                                Ok(13u64)
                            }
                        } else {
                            let a2952 = v2;
                            let a2953 = 90u64;
                            if nat_lt(a2952, a2953) {
                                Ok(3u64)
                            } else {
                                Ok(0u64)
                            }
                        }
                    }
                } else {
                    let a2954 = v2;
                    let a2955 = 94u64;
                    if nat_lt(a2954, a2955) {
                        let a2956 = v2;
                        let a2957 = 92u64;
                        if nat_lt(a2956, a2957) {
                            Ok(10u64)
                        } else {
                            let a2958 = v2;
                            let a2959 = 93u64;
                            if nat_lt(a2958, a2959) {
                                Ok(2u64)
                            } else {
                                Ok(6u64)
                            }
                        }
                    } else {
                        let a2960 = v2;
                        let a2961 = 96u64;
                        if nat_lt(a2960, a2961) {
                            let a2962 = v2;
                            let a2963 = 95u64;
                            if nat_lt(a2962, a2963) {
                                Ok(4u64)
                            } else {
                                Ok(7u64)
                            }
                        } else {
                            let a2964 = v2;
                            let a2965 = 97u64;
                            if nat_lt(a2964, a2965) {
                                Ok(11u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    }
                }
            } else {
                let a2966 = v2;
                let a2967 = 105u64;
                if nat_lt(a2966, a2967) {
                    let a2968 = v2;
                    let a2969 = 101u64;
                    if nat_lt(a2968, a2969) {
                        let a2970 = v2;
                        let a2971 = 99u64;
                        if nat_lt(a2970, a2971) {
                            Ok(5u64)
                        } else {
                            let a2972 = v2;
                            let a2973 = 100u64;
                            if nat_lt(a2972, a2973) {
                                Ok(0u64)
                            } else {
                                Ok(1u64)
                            }
                        }
                    } else {
                        let a2974 = v2;
                        let a2975 = 103u64;
                        if nat_lt(a2974, a2975) {
                            let a2976 = v2;
                            let a2977 = 102u64;
                            if nat_lt(a2976, a2977) {
                                Ok(9u64)
                            } else {
                                Ok(8u64)
                            }
                        } else {
                            let a2978 = v2;
                            let a2979 = 104u64;
                            if nat_lt(a2978, a2979) {
                                Ok(6u64)
                            } else {
                                Ok(14u64)
                            }
                        }
                    }
                } else {
                    let a2980 = v2;
                    let a2981 = 108u64;
                    if nat_lt(a2980, a2981) {
                        let a2982 = v2;
                        let a2983 = 106u64;
                        if nat_lt(a2982, a2983) {
                            Ok(10u64)
                        } else {
                            let a2984 = v2;
                            let a2985 = 107u64;
                            if nat_lt(a2984, a2985) {
                                Ok(2u64)
                            } else {
                                Ok(12u64)
                            }
                        }
                    } else {
                        let a2986 = v2;
                        let a2987 = 110u64;
                        if nat_lt(a2986, a2987) {
                            let a2988 = v2;
                            let a2989 = 109u64;
                            if nat_lt(a2988, a2989) {
                                Ok(3u64)
                            } else {
                                Ok(4u64)
                            }
                        } else {
                            let a2990 = v2;
                            let a2991 = 111u64;
                            if nat_lt(a2990, a2991) {
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

pub fn f132(v0: u64) -> u32 {
    let a2992 = v0;
    let a2993 = 12u64;
    if nat_lt(a2992, a2993) {
        let a2994 = v0;
        let a2995 = 9u64;
        if nat_lt(a2994, a2995) {
            let a2996 = v0;
            let a2997 = 8u64;
            if nat_lt(a2996, a2997) {
                7u32
            } else {
                8u32
            }
        } else {
            let a2998 = v0;
            let a2999 = 10u64;
            if nat_lt(a2998, a2999) {
                9u32
            } else {
                let a3000 = v0;
                let a3001 = 11u64;
                if nat_lt(a3000, a3001) {
                    10u32
                } else {
                    11u32
                }
            }
        }
    } else {
        let a3002 = v0;
        let a3003 = 14u64;
        if nat_lt(a3002, a3003) {
            let a3004 = v0;
            let a3005 = 13u64;
            if nat_lt(a3004, a3005) {
                12u32
            } else {
                13u32
            }
        } else {
            let a3006 = v0;
            let a3007 = 15u64;
            if nat_lt(a3006, a3007) {
                14u32
            } else {
                let a3008 = v0;
                let a3009 = 16u64;
                if nat_lt(a3008, a3009) {
                    15u32
                } else {
                    16u32
                }
            }
        }
    }
}

pub fn f133(v0: Adt5, v1: u64) -> u32 {
    let a3010 = v1;
    let a3011 = 4u64;
    if nat_lt(a3010, a3011) {
        let a3012 = v1;
        let a3013 = 2u64;
        if nat_lt(a3012, a3013) {
            let a3014 = v1;
            let a3015 = 1u64;
            if nat_lt(a3014, a3015) {
                match v0 {
                    Adt5::C0(h3016, _, _, _, _, _, _, _) => {
                        h3016
                    }
                }
            } else {
                match v0 {
                    Adt5::C0(_, h3017, _, _, _, _, _, _) => {
                        h3017
                    }
                }
            }
        } else {
            let a3018 = v1;
            let a3019 = 3u64;
            if nat_lt(a3018, a3019) {
                match v0 {
                    Adt5::C0(_, _, h3020, _, _, _, _, _) => {
                        h3020
                    }
                }
            } else {
                match v0 {
                    Adt5::C0(_, _, _, h3021, _, _, _, _) => {
                        h3021
                    }
                }
            }
        }
    } else {
        let a3022 = v1;
        let a3023 = 6u64;
        if nat_lt(a3022, a3023) {
            let a3024 = v1;
            let a3025 = 5u64;
            if nat_lt(a3024, a3025) {
                match v0 {
                    Adt5::C0(_, _, _, _, h3026, _, _, _) => {
                        h3026
                    }
                }
            } else {
                match v0 {
                    Adt5::C0(_, _, _, _, _, h3027, _, _) => {
                        h3027
                    }
                }
            }
        } else {
            let a3028 = v1;
            let a3029 = 7u64;
            if nat_lt(a3028, a3029) {
                match v0 {
                    Adt5::C0(_, _, _, _, _, _, h3030, _) => {
                        h3030
                    }
                }
            } else {
                match v0 {
                    Adt5::C0(_, _, _, _, _, _, _, h3031) => {
                        h3031
                    }
                }
            }
        }
    }
}

pub fn f134(v0: Adt15, v1: u32, v2: u32) -> u32 {
    let m3032 = v0;
    match m3032 {
        Adt15::C0 => {
            f145(v1, v2)
        }
        Adt15::C1 => {
            f145(v1, v2)
        }
        Adt15::C2 => {
            f146(v1, v2)
        }
        Adt15::C3 => {
            f147(v1, v2)
        }
        Adt15::C4 => {
            f146(v1, v2)
        }
    }
}

pub fn f135(v0: u32, v1: u32) -> Bytes {
    let (_, h3033) = {
        let (_, h3034) = f148(Fn19::F149(v0, v1), 32u64, (0u32, (0u32, Bytes::lit(&[]))));
        h3034
    };
    h3033
}

pub fn f136(v0: u32, v1: u32) -> Bytes {
    let (_, h3035) = f150(Fn20::F151(v0, v1), 32u64, (0u32, Bytes::lit(&[])));
    h3035
}

pub fn f137(v0: u64) -> u32 {
    let a3036 = v0;
    let a3037 = 32u64;
    if nat_lt(a3036, a3037) {
        let a3038 = v0;
        let a3039 = 16u64;
        if nat_lt(a3038, a3039) {
            let a3040 = v0;
            let a3041 = 8u64;
            if nat_lt(a3040, a3041) {
                let a3042 = v0;
                let a3043 = 4u64;
                if nat_lt(a3042, a3043) {
                    let a3044 = v0;
                    let a3045 = 2u64;
                    if nat_lt(a3044, a3045) {
                        let a3046 = v0;
                        let a3047 = 1u64;
                        if nat_lt(a3046, a3047) {
                            1116352408u32
                        } else {
                            1899447441u32
                        }
                    } else {
                        let a3048 = v0;
                        let a3049 = 3u64;
                        if nat_lt(a3048, a3049) {
                            3049323471u32
                        } else {
                            3921009573u32
                        }
                    }
                } else {
                    let a3050 = v0;
                    let a3051 = 6u64;
                    if nat_lt(a3050, a3051) {
                        let a3052 = v0;
                        let a3053 = 5u64;
                        if nat_lt(a3052, a3053) {
                            961987163u32
                        } else {
                            1508970993u32
                        }
                    } else {
                        let a3054 = v0;
                        let a3055 = 7u64;
                        if nat_lt(a3054, a3055) {
                            2453635748u32
                        } else {
                            2870763221u32
                        }
                    }
                }
            } else {
                let a3056 = v0;
                let a3057 = 12u64;
                if nat_lt(a3056, a3057) {
                    let a3058 = v0;
                    let a3059 = 10u64;
                    if nat_lt(a3058, a3059) {
                        let a3060 = v0;
                        let a3061 = 9u64;
                        if nat_lt(a3060, a3061) {
                            3624381080u32
                        } else {
                            310598401u32
                        }
                    } else {
                        let a3062 = v0;
                        let a3063 = 11u64;
                        if nat_lt(a3062, a3063) {
                            607225278u32
                        } else {
                            1426881987u32
                        }
                    }
                } else {
                    let a3064 = v0;
                    let a3065 = 14u64;
                    if nat_lt(a3064, a3065) {
                        let a3066 = v0;
                        let a3067 = 13u64;
                        if nat_lt(a3066, a3067) {
                            1925078388u32
                        } else {
                            2162078206u32
                        }
                    } else {
                        let a3068 = v0;
                        let a3069 = 15u64;
                        if nat_lt(a3068, a3069) {
                            2614888103u32
                        } else {
                            3248222580u32
                        }
                    }
                }
            }
        } else {
            let a3070 = v0;
            let a3071 = 24u64;
            if nat_lt(a3070, a3071) {
                let a3072 = v0;
                let a3073 = 20u64;
                if nat_lt(a3072, a3073) {
                    let a3074 = v0;
                    let a3075 = 18u64;
                    if nat_lt(a3074, a3075) {
                        let a3076 = v0;
                        let a3077 = 17u64;
                        if nat_lt(a3076, a3077) {
                            3835390401u32
                        } else {
                            4022224774u32
                        }
                    } else {
                        let a3078 = v0;
                        let a3079 = 19u64;
                        if nat_lt(a3078, a3079) {
                            264347078u32
                        } else {
                            604807628u32
                        }
                    }
                } else {
                    let a3080 = v0;
                    let a3081 = 22u64;
                    if nat_lt(a3080, a3081) {
                        let a3082 = v0;
                        let a3083 = 21u64;
                        if nat_lt(a3082, a3083) {
                            770255983u32
                        } else {
                            1249150122u32
                        }
                    } else {
                        let a3084 = v0;
                        let a3085 = 23u64;
                        if nat_lt(a3084, a3085) {
                            1555081692u32
                        } else {
                            1996064986u32
                        }
                    }
                }
            } else {
                let a3086 = v0;
                let a3087 = 28u64;
                if nat_lt(a3086, a3087) {
                    let a3088 = v0;
                    let a3089 = 26u64;
                    if nat_lt(a3088, a3089) {
                        let a3090 = v0;
                        let a3091 = 25u64;
                        if nat_lt(a3090, a3091) {
                            2554220882u32
                        } else {
                            2821834349u32
                        }
                    } else {
                        let a3092 = v0;
                        let a3093 = 27u64;
                        if nat_lt(a3092, a3093) {
                            2952996808u32
                        } else {
                            3210313671u32
                        }
                    }
                } else {
                    let a3094 = v0;
                    let a3095 = 30u64;
                    if nat_lt(a3094, a3095) {
                        let a3096 = v0;
                        let a3097 = 29u64;
                        if nat_lt(a3096, a3097) {
                            3336571891u32
                        } else {
                            3584528711u32
                        }
                    } else {
                        let a3098 = v0;
                        let a3099 = 31u64;
                        if nat_lt(a3098, a3099) {
                            113926993u32
                        } else {
                            338241895u32
                        }
                    }
                }
            }
        }
    } else {
        let a3100 = v0;
        let a3101 = 48u64;
        if nat_lt(a3100, a3101) {
            let a3102 = v0;
            let a3103 = 40u64;
            if nat_lt(a3102, a3103) {
                let a3104 = v0;
                let a3105 = 36u64;
                if nat_lt(a3104, a3105) {
                    let a3106 = v0;
                    let a3107 = 34u64;
                    if nat_lt(a3106, a3107) {
                        let a3108 = v0;
                        let a3109 = 33u64;
                        if nat_lt(a3108, a3109) {
                            666307205u32
                        } else {
                            773529912u32
                        }
                    } else {
                        let a3110 = v0;
                        let a3111 = 35u64;
                        if nat_lt(a3110, a3111) {
                            1294757372u32
                        } else {
                            1396182291u32
                        }
                    }
                } else {
                    let a3112 = v0;
                    let a3113 = 38u64;
                    if nat_lt(a3112, a3113) {
                        let a3114 = v0;
                        let a3115 = 37u64;
                        if nat_lt(a3114, a3115) {
                            1695183700u32
                        } else {
                            1986661051u32
                        }
                    } else {
                        let a3116 = v0;
                        let a3117 = 39u64;
                        if nat_lt(a3116, a3117) {
                            2177026350u32
                        } else {
                            2456956037u32
                        }
                    }
                }
            } else {
                let a3118 = v0;
                let a3119 = 44u64;
                if nat_lt(a3118, a3119) {
                    let a3120 = v0;
                    let a3121 = 42u64;
                    if nat_lt(a3120, a3121) {
                        let a3122 = v0;
                        let a3123 = 41u64;
                        if nat_lt(a3122, a3123) {
                            2730485921u32
                        } else {
                            2820302411u32
                        }
                    } else {
                        let a3124 = v0;
                        let a3125 = 43u64;
                        if nat_lt(a3124, a3125) {
                            3259730800u32
                        } else {
                            3345764771u32
                        }
                    }
                } else {
                    let a3126 = v0;
                    let a3127 = 46u64;
                    if nat_lt(a3126, a3127) {
                        let a3128 = v0;
                        let a3129 = 45u64;
                        if nat_lt(a3128, a3129) {
                            3516065817u32
                        } else {
                            3600352804u32
                        }
                    } else {
                        let a3130 = v0;
                        let a3131 = 47u64;
                        if nat_lt(a3130, a3131) {
                            4094571909u32
                        } else {
                            275423344u32
                        }
                    }
                }
            }
        } else {
            let a3132 = v0;
            let a3133 = 56u64;
            if nat_lt(a3132, a3133) {
                let a3134 = v0;
                let a3135 = 52u64;
                if nat_lt(a3134, a3135) {
                    let a3136 = v0;
                    let a3137 = 50u64;
                    if nat_lt(a3136, a3137) {
                        let a3138 = v0;
                        let a3139 = 49u64;
                        if nat_lt(a3138, a3139) {
                            430227734u32
                        } else {
                            506948616u32
                        }
                    } else {
                        let a3140 = v0;
                        let a3141 = 51u64;
                        if nat_lt(a3140, a3141) {
                            659060556u32
                        } else {
                            883997877u32
                        }
                    }
                } else {
                    let a3142 = v0;
                    let a3143 = 54u64;
                    if nat_lt(a3142, a3143) {
                        let a3144 = v0;
                        let a3145 = 53u64;
                        if nat_lt(a3144, a3145) {
                            958139571u32
                        } else {
                            1322822218u32
                        }
                    } else {
                        let a3146 = v0;
                        let a3147 = 55u64;
                        if nat_lt(a3146, a3147) {
                            1537002063u32
                        } else {
                            1747873779u32
                        }
                    }
                }
            } else {
                let a3148 = v0;
                let a3149 = 60u64;
                if nat_lt(a3148, a3149) {
                    let a3150 = v0;
                    let a3151 = 58u64;
                    if nat_lt(a3150, a3151) {
                        let a3152 = v0;
                        let a3153 = 57u64;
                        if nat_lt(a3152, a3153) {
                            1955562222u32
                        } else {
                            2024104815u32
                        }
                    } else {
                        let a3154 = v0;
                        let a3155 = 59u64;
                        if nat_lt(a3154, a3155) {
                            2227730452u32
                        } else {
                            2361852424u32
                        }
                    }
                } else {
                    let a3156 = v0;
                    let a3157 = 62u64;
                    if nat_lt(a3156, a3157) {
                        let a3158 = v0;
                        let a3159 = 61u64;
                        if nat_lt(a3158, a3159) {
                            2428436474u32
                        } else {
                            2756734187u32
                        }
                    } else {
                        let a3160 = v0;
                        let a3161 = 63u64;
                        if nat_lt(a3160, a3161) {
                            3204031479u32
                        } else {
                            3329325298u32
                        }
                    }
                }
            }
        }
    }
}

pub fn f138(v0: u32) -> u32 {
    let a3164 = {
        let a3162 = f152(v0, 6u32, 26u32);
        let a3163 = f152(v0, 11u32, 21u32);
        fixed_u32::bit_xor(a3162, a3163)
    };
    let a3165 = f152(v0, 25u32, 7u32);
    fixed_u32::bit_xor(a3164, a3165)
}

pub fn f139(v0: u32, v1: u32, v2: u32) -> u32 {
    let a3171 = {
        let a3166 = v0;
        let a3167 = v1;
        fixed_u32::bit_and(a3166, a3167)
    };
    let a3172 = {
        let a3169 = {
            let a3168 = v0;
            fixed_u32::bit_not(a3168)
        };
        let a3170 = v2;
        fixed_u32::bit_and(a3169, a3170)
    };
    fixed_u32::bit_xor(a3171, a3172)
}

pub fn f140(v0: u32) -> u32 {
    let a3175 = {
        let a3173 = f152(v0, 2u32, 30u32);
        let a3174 = f152(v0, 13u32, 19u32);
        fixed_u32::bit_xor(a3173, a3174)
    };
    let a3176 = f152(v0, 22u32, 10u32);
    fixed_u32::bit_xor(a3175, a3176)
}

pub fn f141(v0: u32, v1: u32, v2: u32) -> u32 {
    let a3185 = {
        let a3181 = {
            let a3177 = v0;
            let a3178 = v1;
            fixed_u32::bit_and(a3177, a3178)
        };
        let a3182 = {
            let a3179 = v0;
            let a3180 = v2;
            fixed_u32::bit_and(a3179, a3180)
        };
        fixed_u32::bit_xor(a3181, a3182)
    };
    let a3186 = {
        let a3183 = v1;
        let a3184 = v2;
        fixed_u32::bit_and(a3183, a3184)
    };
    fixed_u32::bit_xor(a3185, a3186)
}

pub fn f142(v0: Adt17) -> Adt17 {
    Adt17::C0(match v0.clone() {
        Adt17::C0(_, h3187, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h3187
        }
    }, match v0.clone() {
        Adt17::C0(_, _, h3188, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h3188
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, h3189, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h3189
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, _, h3190, _, _, _, _, _, _, _, _, _, _, _) => {
            h3190
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, _, _, h3191, _, _, _, _, _, _, _, _, _, _) => {
            h3191
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, _, _, _, h3192, _, _, _, _, _, _, _, _, _) => {
            h3192
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, _, _, _, _, h3193, _, _, _, _, _, _, _, _) => {
            h3193
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, _, _, _, _, _, h3194, _, _, _, _, _, _, _) => {
            h3194
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, _, _, _, _, _, _, h3195, _, _, _, _, _, _) => {
            h3195
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, _, _, _, _, _, _, _, h3196, _, _, _, _, _) => {
            h3196
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, _, _, _, _, _, _, _, _, h3197, _, _, _, _) => {
            h3197
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, _, _, _, _, _, _, _, _, _, h3198, _, _, _) => {
            h3198
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h3199, _, _) => {
            h3199
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h3200, _) => {
            h3200
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h3201) => {
            h3201
        }
    }, f116(f153(match v0.clone() {
        Adt17::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h3202, _) => {
            h3202
        }
    }), f116(match v0.clone() {
        Adt17::C0(_, _, _, _, _, _, _, _, _, h3203, _, _, _, _, _, _) => {
            h3203
        }
    }, f116(f154(match v0.clone() {
        Adt17::C0(_, h3204, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h3204
        }
    }), match v0 {
        Adt17::C0(h3205, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h3205
        }
    }))))
}

pub fn f143(v0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let v1: u64 = {
        let (h3206, _) = v0;
        h3206
    };
    let v2: u32 = {
        let (h3207, _) = {
            let (_, h3208) = v0;
            h3208
        };
        h3207
    };
    let v3: u32 = {
        let (_, h3209) = {
            let (_, h3210) = v0;
            h3210
        };
        h3209
    };
    ({
        let a3211 = v1;
        let a3212 = 2u64;
        let a3213 = 0u64;
        nat_quot(a3211, a3212, a3213)
    }, (f144(v2, 1u32), {
        let a3217 = {
            let a3214 = v1;
            let a3215 = 2u64;
            let a3216 = 0u64;
            nat_rem(a3214, a3215, a3216)
        };
        let a3218 = 1u64;
        if nat_eq(a3217, a3218) {
            let a3219 = v3;
            let a3220 = v2;
            fixed_u32::bit_or(a3219, a3220)
        } else {
            v3
        }
    }))
}

pub fn f144(v0: u32, v1: u32) -> u32 {
    let m3223 = {
        let a3221 = v0;
        let a3222 = v1;
        fixed_u32::shift_left(a3221, a3222)
    };
    match m3223 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f145(v0: u32, v1: u32) -> u32 {
    let m3226 = {
        let a3224 = f155(v0);
        let a3225 = f155(v1);
        fixed_u64::checked_add(a3224, a3225)
    };
    match m3226 {
        None => {
            0u32
        }
        Some(v2) => {
            f156(v2)
        }
    }
}

pub fn f146(v0: u32, v1: u32) -> u32 {
    let a3227 = v0;
    let a3228 = v1;
    fixed_u32::bit_xor(a3227, a3228)
}

pub fn f147(v0: u32, v1: u32) -> u32 {
    let m3231 = {
        let a3229 = 32u32;
        let a3230 = v1;
        fixed_u32::checked_sub(a3229, a3230)
    };
    match m3231 {
        None => {
            v0
        }
        Some(v2) => {
            f157(v0, v1, v2)
        }
    }
}

pub fn f148(v0: Fn19, p3235: u64, p3236: (u32, (u32, Bytes))) -> (u32, (u32, Bytes)) {
    let m3234 = (p3235, p3236);
    {
        let mut m3234 = m3234;
        loop {
            let step = {
                let (v1, v2) = m3234;
                let m3232 = v1;
                if m3232 == 0 {
                    Err::<(u64, (u32, (u32, Bytes))), (u32, (u32, Bytes))>(v2)
                } else {
                    let v3 = m3232 - 1;
                    Ok::<(u64, (u32, (u32, Bytes))), (u32, (u32, Bytes))>((v3, {
                        let c3233: Fn19 = v0.clone();
                        c3233.apply(v2)
                    }))
                }
            };
            match step {
                Ok(next) => { m3234 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f149(v0: u32, v1: u32, v2: (u32, (u32, Bytes))) -> (u32, (u32, Bytes)) {
    let v3: u32 = {
        let (h3237, _) = v2.clone();
        h3237
    };
    let v4: u32 = f158(v0, v3);
    let v5: u32 = f158(v1, v3);
    let v6: u32 = {
        let (h3238, _) = {
            let (_, h3239) = v2.clone();
            h3239
        };
        h3238
    };
    (f145(v3, 1u32), (f159(v4, v5, v6), {
        let a3242 = {
            let (_, h3240) = {
                let (_, h3241) = v2;
                h3241
            };
            h3240
        };
        let a3243 = f160(f161(v4, v5, v6));
        append_bytes(a3242, a3243)
    }))
}

pub fn f150(v0: Fn20, p3247: u64, p3248: (u32, Bytes)) -> (u32, Bytes) {
    let m3246 = (p3247, p3248);
    {
        let mut m3246 = m3246;
        loop {
            let step = {
                let (v1, v2) = m3246;
                let m3244 = v1;
                if m3244 == 0 {
                    Err::<(u64, (u32, Bytes)), (u32, Bytes)>(v2)
                } else {
                    let v3 = m3244 - 1;
                    Ok::<(u64, (u32, Bytes)), (u32, Bytes)>((v3, {
                        let c3245: Fn20 = v0.clone();
                        c3245.apply(v2)
                    }))
                }
            };
            match step {
                Ok(next) => { m3246 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f151(v0: u32, v1: u32, v2: (u32, Bytes)) -> (u32, Bytes) {
    let v3: u32 = {
        let (h3249, _) = v2;
        h3249
    };
    (f145(v3, 1u32), {
        let a3251 = {
            let (_, h3250) = v2;
            h3250
        };
        let a3252 = f160(f162(f158(v0, v3), f158(v1, v3)));
        append_bytes(a3251, a3252)
    })
}

pub fn f152(v0: u32, v1: u32, v2: u32) -> u32 {
    let a3253 = f163(v0, v1);
    let a3254 = f144(v0, v2);
    fixed_u32::bit_or(a3253, a3254)
}

pub fn f153(v0: u32) -> u32 {
    let a3257 = {
        let a3255 = f152(v0, 17u32, 15u32);
        let a3256 = f152(v0, 19u32, 13u32);
        fixed_u32::bit_xor(a3255, a3256)
    };
    let a3258 = f163(v0, 10u32);
    fixed_u32::bit_xor(a3257, a3258)
}

pub fn f154(v0: u32) -> u32 {
    let a3261 = {
        let a3259 = f152(v0, 7u32, 25u32);
        let a3260 = f152(v0, 18u32, 14u32);
        fixed_u32::bit_xor(a3259, a3260)
    };
    let a3262 = f163(v0, 3u32);
    fixed_u32::bit_xor(a3261, a3262)
}

pub fn f155(v0: u32) -> u64 {
    let m3264 = {
        let a3263 = v0;
        fixed_u64::convert(i128::from(a3263))
    };
    match m3264 {
        None => {
            0u64
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f156(v0: u64) -> u32 {
    let m3268 = {
        let a3267 = {
            let a3265 = v0;
            let a3266 = 4294967295u64;
            fixed_u64::bit_and(a3265, a3266)
        };
        fixed_u32::convert(i128::from(a3267))
    };
    match m3268 {
        None => {
            0u32
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f157(v0: u32, v1: u32, v2: u32) -> u32 {
    let a3269 = f32(v0, v1);
    let a3270 = f52(v0, v2);
    fixed_u32::bit_or(a3269, a3270)
}

pub fn f158(v0: u32, v1: u32) -> u32 {
    let a3271 = f32(v0, v1);
    let a3272 = 1u32;
    fixed_u32::bit_and(a3271, a3272)
}

pub fn f159(v0: u32, v1: u32, v2: u32) -> u32 {
    let a3281 = {
        let a3277 = {
            let a3273 = v0;
            let a3274 = v1;
            fixed_u32::bit_and(a3273, a3274)
        };
        let a3278 = {
            let a3275 = v0;
            let a3276 = v2;
            fixed_u32::bit_and(a3275, a3276)
        };
        fixed_u32::bit_or(a3277, a3278)
    };
    let a3282 = {
        let a3279 = v1;
        let a3280 = v2;
        fixed_u32::bit_and(a3279, a3280)
    };
    fixed_u32::bit_or(a3281, a3282)
}

pub fn f160(v0: u32) -> Bytes {
    let a3285 = {
        let a3283 = v0;
        let a3284 = 16u32;
        fixed_u32::bit_and(a3283, a3284)
    };
    let a3286 = 0u32;
    if equal(a3285, a3286) {
        let a3289 = {
            let a3287 = v0;
            let a3288 = 8u32;
            fixed_u32::bit_and(a3287, a3288)
        };
        let a3290 = 0u32;
        if equal(a3289, a3290) {
            let a3293 = {
                let a3291 = v0;
                let a3292 = 4u32;
                fixed_u32::bit_and(a3291, a3292)
            };
            let a3294 = 0u32;
            if equal(a3293, a3294) {
                let a3297 = {
                    let a3295 = v0;
                    let a3296 = 2u32;
                    fixed_u32::bit_and(a3295, a3296)
                };
                let a3298 = 0u32;
                if equal(a3297, a3298) {
                    let a3301 = {
                        let a3299 = v0;
                        let a3300 = 1u32;
                        fixed_u32::bit_and(a3299, a3300)
                    };
                    let a3302 = 0u32;
                    if equal(a3301, a3302) {
                        Bytes::lit(&[0x00])
                    } else {
                        Bytes::lit(&[0x01])
                    }
                } else {
                    let a3305 = {
                        let a3303 = v0;
                        let a3304 = 1u32;
                        fixed_u32::bit_and(a3303, a3304)
                    };
                    let a3306 = 0u32;
                    if equal(a3305, a3306) {
                        Bytes::lit(&[0x02])
                    } else {
                        Bytes::lit(&[0x03])
                    }
                }
            } else {
                let a3309 = {
                    let a3307 = v0;
                    let a3308 = 2u32;
                    fixed_u32::bit_and(a3307, a3308)
                };
                let a3310 = 0u32;
                if equal(a3309, a3310) {
                    let a3313 = {
                        let a3311 = v0;
                        let a3312 = 1u32;
                        fixed_u32::bit_and(a3311, a3312)
                    };
                    let a3314 = 0u32;
                    if equal(a3313, a3314) {
                        Bytes::lit(&[0x04])
                    } else {
                        Bytes::lit(&[0x05])
                    }
                } else {
                    let a3317 = {
                        let a3315 = v0;
                        let a3316 = 1u32;
                        fixed_u32::bit_and(a3315, a3316)
                    };
                    let a3318 = 0u32;
                    if equal(a3317, a3318) {
                        Bytes::lit(&[0x06])
                    } else {
                        Bytes::lit(&[0x07])
                    }
                }
            }
        } else {
            let a3321 = {
                let a3319 = v0;
                let a3320 = 4u32;
                fixed_u32::bit_and(a3319, a3320)
            };
            let a3322 = 0u32;
            if equal(a3321, a3322) {
                let a3325 = {
                    let a3323 = v0;
                    let a3324 = 2u32;
                    fixed_u32::bit_and(a3323, a3324)
                };
                let a3326 = 0u32;
                if equal(a3325, a3326) {
                    let a3329 = {
                        let a3327 = v0;
                        let a3328 = 1u32;
                        fixed_u32::bit_and(a3327, a3328)
                    };
                    let a3330 = 0u32;
                    if equal(a3329, a3330) {
                        Bytes::lit(&[0x08])
                    } else {
                        Bytes::lit(&[0x09])
                    }
                } else {
                    let a3333 = {
                        let a3331 = v0;
                        let a3332 = 1u32;
                        fixed_u32::bit_and(a3331, a3332)
                    };
                    let a3334 = 0u32;
                    if equal(a3333, a3334) {
                        Bytes::lit(&[0x0a])
                    } else {
                        Bytes::lit(&[0x0b])
                    }
                }
            } else {
                let a3337 = {
                    let a3335 = v0;
                    let a3336 = 2u32;
                    fixed_u32::bit_and(a3335, a3336)
                };
                let a3338 = 0u32;
                if equal(a3337, a3338) {
                    let a3341 = {
                        let a3339 = v0;
                        let a3340 = 1u32;
                        fixed_u32::bit_and(a3339, a3340)
                    };
                    let a3342 = 0u32;
                    if equal(a3341, a3342) {
                        Bytes::lit(&[0x0c])
                    } else {
                        Bytes::lit(&[0x0d])
                    }
                } else {
                    let a3345 = {
                        let a3343 = v0;
                        let a3344 = 1u32;
                        fixed_u32::bit_and(a3343, a3344)
                    };
                    let a3346 = 0u32;
                    if equal(a3345, a3346) {
                        Bytes::lit(&[0x0e])
                    } else {
                        Bytes::lit(&[0x0f])
                    }
                }
            }
        }
    } else {
        let a3349 = {
            let a3347 = v0;
            let a3348 = 8u32;
            fixed_u32::bit_and(a3347, a3348)
        };
        let a3350 = 0u32;
        if equal(a3349, a3350) {
            let a3353 = {
                let a3351 = v0;
                let a3352 = 4u32;
                fixed_u32::bit_and(a3351, a3352)
            };
            let a3354 = 0u32;
            if equal(a3353, a3354) {
                let a3357 = {
                    let a3355 = v0;
                    let a3356 = 2u32;
                    fixed_u32::bit_and(a3355, a3356)
                };
                let a3358 = 0u32;
                if equal(a3357, a3358) {
                    let a3361 = {
                        let a3359 = v0;
                        let a3360 = 1u32;
                        fixed_u32::bit_and(a3359, a3360)
                    };
                    let a3362 = 0u32;
                    if equal(a3361, a3362) {
                        Bytes::lit(&[0x10])
                    } else {
                        Bytes::lit(&[0x11])
                    }
                } else {
                    let a3365 = {
                        let a3363 = v0;
                        let a3364 = 1u32;
                        fixed_u32::bit_and(a3363, a3364)
                    };
                    let a3366 = 0u32;
                    if equal(a3365, a3366) {
                        Bytes::lit(&[0x12])
                    } else {
                        Bytes::lit(&[0x13])
                    }
                }
            } else {
                let a3369 = {
                    let a3367 = v0;
                    let a3368 = 2u32;
                    fixed_u32::bit_and(a3367, a3368)
                };
                let a3370 = 0u32;
                if equal(a3369, a3370) {
                    let a3373 = {
                        let a3371 = v0;
                        let a3372 = 1u32;
                        fixed_u32::bit_and(a3371, a3372)
                    };
                    let a3374 = 0u32;
                    if equal(a3373, a3374) {
                        Bytes::lit(&[0x14])
                    } else {
                        Bytes::lit(&[0x15])
                    }
                } else {
                    let a3377 = {
                        let a3375 = v0;
                        let a3376 = 1u32;
                        fixed_u32::bit_and(a3375, a3376)
                    };
                    let a3378 = 0u32;
                    if equal(a3377, a3378) {
                        Bytes::lit(&[0x16])
                    } else {
                        Bytes::lit(&[0x17])
                    }
                }
            }
        } else {
            let a3381 = {
                let a3379 = v0;
                let a3380 = 4u32;
                fixed_u32::bit_and(a3379, a3380)
            };
            let a3382 = 0u32;
            if equal(a3381, a3382) {
                let a3385 = {
                    let a3383 = v0;
                    let a3384 = 2u32;
                    fixed_u32::bit_and(a3383, a3384)
                };
                let a3386 = 0u32;
                if equal(a3385, a3386) {
                    let a3389 = {
                        let a3387 = v0;
                        let a3388 = 1u32;
                        fixed_u32::bit_and(a3387, a3388)
                    };
                    let a3390 = 0u32;
                    if equal(a3389, a3390) {
                        Bytes::lit(&[0x18])
                    } else {
                        Bytes::lit(&[0x19])
                    }
                } else {
                    let a3393 = {
                        let a3391 = v0;
                        let a3392 = 1u32;
                        fixed_u32::bit_and(a3391, a3392)
                    };
                    let a3394 = 0u32;
                    if equal(a3393, a3394) {
                        Bytes::lit(&[0x1a])
                    } else {
                        Bytes::lit(&[0x1b])
                    }
                }
            } else {
                let a3397 = {
                    let a3395 = v0;
                    let a3396 = 2u32;
                    fixed_u32::bit_and(a3395, a3396)
                };
                let a3398 = 0u32;
                if equal(a3397, a3398) {
                    let a3401 = {
                        let a3399 = v0;
                        let a3400 = 1u32;
                        fixed_u32::bit_and(a3399, a3400)
                    };
                    let a3402 = 0u32;
                    if equal(a3401, a3402) {
                        Bytes::lit(&[0x1c])
                    } else {
                        Bytes::lit(&[0x1d])
                    }
                } else {
                    let a3405 = {
                        let a3403 = v0;
                        let a3404 = 1u32;
                        fixed_u32::bit_and(a3403, a3404)
                    };
                    let a3406 = 0u32;
                    if equal(a3405, a3406) {
                        Bytes::lit(&[0x1e])
                    } else {
                        Bytes::lit(&[0x1f])
                    }
                }
            }
        }
    }
}

pub fn f161(v0: u32, v1: u32, v2: u32) -> u32 {
    let v3: u32 = {
        let a3409 = {
            let a3407 = v0;
            let a3408 = v1;
            fixed_u32::bit_xor(a3407, a3408)
        };
        let a3410 = v2;
        fixed_u32::bit_xor(a3409, a3410)
    };
    let v4: u32 = {
        let a3419 = {
            let a3415 = {
                let a3411 = v0;
                let a3412 = v1;
                fixed_u32::bit_and(a3411, a3412)
            };
            let a3416 = {
                let a3413 = v0;
                let a3414 = v2;
                fixed_u32::bit_and(a3413, a3414)
            };
            fixed_u32::bit_or(a3415, a3416)
        };
        let a3420 = {
            let a3417 = v1;
            let a3418 = v2;
            fixed_u32::bit_and(a3417, a3418)
        };
        fixed_u32::bit_or(a3419, a3420)
    };
    let a3427 = v0;
    let a3428 = {
        let a3425 = f52(v1, 1u32);
        let a3426 = {
            let a3423 = f52(v2, 2u32);
            let a3424 = {
                let a3421 = f52(v3, 3u32);
                let a3422 = f52(v4, 4u32);
                fixed_u32::bit_or(a3421, a3422)
            };
            fixed_u32::bit_or(a3423, a3424)
        };
        fixed_u32::bit_or(a3425, a3426)
    };
    fixed_u32::bit_or(a3427, a3428)
}

pub fn f162(v0: u32, v1: u32) -> u32 {
    let a3433 = v0;
    let a3434 = {
        let a3431 = f52(v1, 1u32);
        let a3432 = f52({
            let a3429 = v0;
            let a3430 = v1;
            fixed_u32::bit_xor(a3429, a3430)
        }, 2u32);
        fixed_u32::bit_or(a3431, a3432)
    };
    fixed_u32::bit_or(a3433, a3434)
}

pub fn f163(v0: u32, v1: u32) -> u32 {
    let m3437 = {
        let a3435 = v0;
        let a3436 = v1;
        fixed_u32::shift_right(a3435, a3436)
    };
    match m3437 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn run(p0: Adt0, p1: List<Bytes>, p2: u64, p3: u64) -> R<Result<Bytes, Adt1>> {
    f0(p0, p1, p2, p3)
}
