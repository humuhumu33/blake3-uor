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
    F10,
}

impl Fn0 {
    pub fn apply(&self, p0: List<Bytes>) -> Option<List<Bytes>> {
        match self {
            Fn0::F10 => f10(p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn1 {
    F14,
}

impl Fn1 {
    pub fn apply(&self, p0: Adt9, p1: Bytes) -> R<Adt9> {
        match self {
            Fn1::F14 => f14(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn2 {
    F17,
}

impl Fn2 {
    pub fn apply(&self, p0: Adt2, p1: Adt3) -> R<Adt2> {
        match self {
            Fn2::F17 => f17(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn3 {
    F27(u32, Adt5),
}

impl Fn3 {
    pub fn apply(&self, p0: (Adt10, (List<Adt3>, u64)), p1: Adt7) -> R<(Adt10, (List<Adt3>, u64))> {
        match self {
            Fn3::F27(k0, k1) => f27(*k0, k1.clone(), p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn4 {
    F29((Adt10, (List<Adt3>, u64))),
}

impl Fn4 {
    pub fn apply(&self, p0: (u64, List<Adt3>)) -> R<(u64, List<Adt3>)> {
        match self {
            Fn4::F29(k0) => f29(k0.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn5 {
    F43,
}

impl Fn5 {
    pub fn apply(&self, p0: (Option<Bytes>, List<Bytes>), p1: Bytes) -> (Option<Bytes>, List<Bytes>) {
        match self {
            Fn5::F43 => f43(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn6 {
    F45,
    F69,
    F74,
}

impl Fn6 {
    pub fn apply(&self, p0: List<Bytes>, p1: Bytes) -> List<Bytes> {
        match self {
            Fn6::F45 => f45(p0, p1),
            Fn6::F69 => f69(p0, p1),
            Fn6::F74 => f74(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn7 {
    F53(u32, u64, u64, Adt5, u64, Bytes),
}

impl Fn7 {
    pub fn apply(&self, p0: (u64, (Adt5, List<Adt3>))) -> R<(u64, (Adt5, List<Adt3>))> {
        match self {
            Fn7::F53(k0, k1, k2, k3, k4, k5) => f53(*k0, *k1, *k2, k3.clone(), *k4, k5.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn8 {
    F61,
}

impl Fn8 {
    pub fn apply(&self, p0: List<Adt3>, p1: Adt3) -> List<Adt3> {
        match self {
            Fn8::F61 => f61(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn9 {
    F63(u32, Bytes, Adt5, Adt9),
}

impl Fn9 {
    pub fn apply(&self, p0: (u64, (List<Adt7>, List<Adt3>))) -> R<(u64, (List<Adt7>, List<Adt3>))> {
        match self {
            Fn9::F63(k0, k1, k2, k3) => f63(*k0, k1.clone(), k2.clone(), k3.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn10 {
    F73,
}

impl Fn10 {
    pub fn apply(&self, p0: List<Bytes>, p1: Adt3) -> List<Bytes> {
        match self {
            Fn10::F73 => f73(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn11 {
    F80(Bytes, u64, u64),
}

impl Fn11 {
    pub fn apply(&self, p0: (u64, Adt11)) -> R<(u64, Adt11)> {
        match self {
            Fn11::F80(k0, k1, k2) => f80(k0.clone(), *k1, *k2, p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn12 {
    F85(u64),
}

impl Fn12 {
    pub fn apply(&self, p0: Adt6, p1: Adt3) -> Adt6 {
        match self {
            Fn12::F85(k0) => f85(*k0, p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn13 {
    F93,
    F143,
}

impl Fn13 {
    pub fn apply(&self, p0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
        match self {
            Fn13::F93 => f93(p0),
            Fn13::F143 => f143(p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn14 {
    F97(u64),
}

impl Fn14 {
    pub fn apply(&self, p0: List<Bytes>, p1: Adt14) -> List<Bytes> {
        match self {
            Fn14::F97(k0) => f97(*k0, p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn15 {
    F104(u32, Adt5),
}

impl Fn15 {
    pub fn apply(&self, p0: Adt12) -> R<Option<Adt12>> {
        match self {
            Fn15::F104(k0, k1) => f104(*k0, k1.clone(), p0),
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

pub fn f0(v0: Adt0, v1: List<Bytes>, v2: u64) -> R<Result<Bytes, Adt1>> {
    let m1 = f1(v0, v1, v2, 0u64)?;
    match m1 {
        Ok(v3) => {
            Ok(Ok::<Bytes, Adt1>({
                let m5 = {
                    let m4 = {
                        let (h3, _) = v3;
                        h3
                    };
                    match m4 {
                        Adt2::C0(_, h2, _, _, _, _, _, _, _, _, _) => {
                            h2
                        }
                    }
                };
                match m5.uncons() {
                    None => {
                        Bytes::lit(&[])
                    }
                    Some((v4, _)) => {
                        let m12 = {
                            let a9 = v4.clone();
                            let a10 = {
                                let a7 = {
                                    let a6 = v4;
                                    length_bytes(a6)
                                };
                                let a8 = 64u64;
                                nat_sub(a7, a8)
                            };
                            let a11 = 64u64;
                            slice_bytes(a9, a10, a11)
                        };
                        match m12 {
                            None => {
                                Bytes::lit(&[])
                            }
                            Some(v6) => {
                                v6
                            }
                        }
                    }
                }
            }))
        }
        Err(v7) => {
            Ok(Err::<Bytes, Adt1>(v7))
        }
    }
}

pub fn f1(v0: Adt0, v1: List<Bytes>, v2: u64, v3: u64) -> R<Result<(Adt2, Bytes), Adt1>> {
    let v4: Bytes = f2(v1.clone());
    let m13 = f3(v0.clone(), v1, v2)?;
    match m13 {
        Ok(v5) => {
            Ok(Ok::<(Adt2, Bytes), Adt1>({
                let m14 = v0;
                match m14 {
                    Adt0::C0 => {
                        f4(f5(v3, List::<Bytes>::nil(), 0u64), v5, f8(v4)?, Bytes::lit(&[]))?
                    }
                    Adt0::C1(_) => {
                        f4(f5(v3, List::<Bytes>::nil(), 0u64), v5, f8(v4)?, Bytes::lit(&[]))?
                    }
                    Adt0::C2(v6) => {
                        let v7: (Adt2, Bytes) = f4(f5(v3, List::<Bytes>::nil(), 0u64), Adt8::C0(3u64, {
                            let a15 = v6.clone();
                            length_bytes(a15)
                        }, 32u64, f6(), f7(), List::<Adt3>::nil(), match v5.clone() {
                            Adt8::C0(_, _, _, _, _, h16, _) => {
                                h16
                            }
                        }), f8(v6)?, Bytes::lit(&[]))?;
                        f4({
                            let (h17, _) = v7;
                            h17
                        }, v5, f8(v4)?, {
                            let (_, h18) = v7;
                            h18
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

pub fn f2(v0: List<Bytes>) -> Bytes {
    let m20 = {
        let (h19, _) = f9(Fn0::F10, 64u64, v0);
        h19
    };
    match m20.uncons() {
        None => {
            Bytes::lit(&[])
        }
        Some((v1, _)) => {
            v1
        }
    }
}

pub fn f3(v0: Adt0, v1: List<Bytes>, v2: u64) -> R<Result<Adt8, Adt1>> {
    let m21 = f11(v0)?;
    match m21 {
        Ok(v3) => {
            Ok(Ok::<Adt8, Adt1>(f12(f13(Fn1::F14, v3, v1)?, v2)?))
        }
        Err(v4) => {
            Ok(Err::<Adt8, Adt1>(v4))
        }
    }
}

pub fn f4(v0: Adt2, v1: Adt8, v2: Bytes, v3: Bytes) -> R<(Adt2, Bytes)> {
    f15(f16(Fn2::F17, f5(match v0.clone() {
        Adt2::C0(h22, _, _, _, _, _, _, _, _, _, _) => {
            h22
        }
    }, match v0.clone() {
        Adt2::C0(_, h23, _, _, _, _, _, _, _, _, _) => {
            h23
        }
    }, match v0 {
        Adt2::C0(_, _, h24, _, _, _, _, _, _, _, _) => {
            h24
        }
    }), match v1.clone() {
        Adt8::C0(_, _, _, _, _, _, h25) => {
            h25
        }
    })?, v1, v2, v3)
}

pub fn f5(v0: u64, v1: List<Bytes>, v2: u64) -> Adt2 {
    Adt2::C0(v0, v1, v2, List::<Bytes>::nil(), List::<Bytes>::nil(), 0u64, None::<Bytes>, Bytes::lit(&[]), List::<Bytes>::nil(), List::<Bytes>::nil(), 0u64)
}

pub fn f6() -> Adt5 {
    Adt5::C0(1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, 1359893119u32, 2600822924u32, 528734635u32, 1541459225u32)
}

pub fn f7() -> u32 {
    32u32
}

pub fn f8(v0: Bytes) -> R<Bytes> {
    let a26 = f18(v0.clone(), 32u64)?;
    let a27 = f19(v0)?;
    Ok(append_bytes(a26, a27))
}

pub fn f9(v0: Fn0, p32: u64, p33: List<Bytes>) -> (List<Bytes>, bool) {
    let m31 = (p32, p33);
    {
        let mut m31 = m31;
        loop {
            let step = {
                let (v1, v2) = m31;
                let m28 = v1;
                if m28 == 0 {
                    Err::<(u64, List<Bytes>), (List<Bytes>, bool)>((v2, false))
                } else {
                    let v3 = m28 - 1;
                    let m30 = {
                        let c29: Fn0 = v0.clone();
                        c29.apply(v2.clone())
                    };
                    match m30 {
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
                Ok(next) => { m31 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f10(v0: List<Bytes>) -> Option<List<Bytes>> {
    let a35 = {
        let a34 = v0.clone();
        length_list(a34)
    };
    let a36 = 1u64;
    if nat_le(a35, a36) {
        None::<List<Bytes>>
    } else {
        Some(f20(v0))
    }
}

pub fn f11(v0: Adt0) -> R<Result<Adt9, Adt1>> {
    let m37 = v0;
    match m37 {
        Adt0::C0 => {
            Ok(Ok::<Adt9, Adt1>(f21(f6(), 0u32)))
        }
        Adt0::C1(v1) => {
            let a39 = {
                let a38 = v1.clone();
                length_bytes(a38)
            };
            let a40 = 32u64;
            if nat_eq(a39, a40) {
                Ok(Ok::<Adt9, Adt1>(f21(f22(v1)?, f23())))
            } else {
                Ok(Err::<Adt9, Adt1>(Adt1::C0({
                    let a41 = v1;
                    length_bytes(a41)
                })))
            }
        }
        Adt0::C2(v2) => {
            Ok(Ok::<Adt9, Adt1>(f24(v2)?))
        }
    }
}

pub fn f12(v0: Adt9, v1: u64) -> R<Adt8> {
    let v2: Adt5 = match v0.clone() {
        Adt9::C0(h42, _, _, _, _, _, _) => {
            h42
        }
    };
    let v3: u32 = match v0.clone() {
        Adt9::C0(_, h43, _, _, _, _, _) => {
            h43
        }
    };
    let v4: Bytes = match v0.clone() {
        Adt9::C0(_, _, _, h44, _, _, _) => {
            h44
        }
    };
    let v5: (Adt10, List<Adt3>) = f25(v2.clone(), v3, v4.clone(), 0u64, {
        let a45 = v4.clone();
        length_bytes(a45)
    }, match v0.clone() {
        Adt9::C0(_, _, h46, _, _, _, _) => {
            h46
        }
    }, match v0.clone() {
        Adt9::C0(_, _, _, _, _, h47, _) => {
            h47
        }
    })?;
    let v6: (Adt10, (List<Adt3>, u64)) = f26(Fn3::F27(v3, v2.clone()), ({
        let (h48, _) = v5;
        h48
    }, ({
        let (_, h49) = v5;
        h49
    }, {
        let a51 = match v0.clone() {
            Adt9::C0(_, _, _, _, h50, _, _) => {
                h50
            }
        };
        length_list(a51)
    })), match v0.clone() {
        Adt9::C0(_, _, _, _, h52, _, _) => {
            h52
        }
    })?;
    let v7: (u64, List<Adt3>) = f28(Fn4::F29(v6.clone()), {
        let a55 = {
            let a53 = v1;
            let a54 = 63u64;
            nat_add(a53, a54)?
        };
        let a56 = 64u64;
        let a57 = 0u64;
        nat_quot(a55, a56, a57)
    }, (0u64, {
        let (h58, _) = {
            let (_, h59) = v6;
            h59
        };
        h58
    }))?;
    Ok(Adt8::C0(f30(v3), {
        let a64 = {
            let a61 = match v0.clone() {
                Adt9::C0(_, _, h60, _, _, _, _) => {
                    h60
                }
            };
            let a62 = 1024u64;
            nat_mul(a61, a62)?
        };
        let a65 = {
            let a63 = v4;
            length_bytes(a63)
        };
        nat_add(a64, a65)?
    }, v1, v2, v3, match v0 {
        Adt9::C0(_, _, _, _, _, _, h66) => {
            h66
        }
    }, f31({
        let (_, h67) = v7;
        h67
    })))
}

pub fn f13(v0: Fn1, p71: Adt9, p72: List<Bytes>) -> R<Adt9> {
    let m70 = (p71, p72);
    {
        let mut m70 = m70;
        loop {
            let step = {
                let (v1, v2) = m70;
                let m68 = v2;
                match m68.uncons() {
                    None => {
                        Err::<(Adt9, List<Bytes>), R<Adt9>>(Ok(v1))
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt9, List<Bytes>), R<Adt9>>(({
                            let c69: Fn1 = v0.clone();
                            c69.apply(v1, v3)?
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m70 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f14(v0: Adt9, v1: Bytes) -> R<Adt9> {
    f32(v0, v1)
}

pub fn f15(v0: Adt2, v1: Adt8, v2: Bytes, v3: Bytes) -> R<(Adt2, Bytes)> {
    let v4: u64 = {
        let a74 = match v1.clone() {
            Adt8::C0(_, h73, _, _, _, _, _) => {
                h73
            }
        };
        let a75 = 0u64;
        if nat_eq(a74, a75) {
            1u64
        } else {
            let a79 = {
                let a77 = match v1.clone() {
                    Adt8::C0(_, h76, _, _, _, _, _) => {
                        h76
                    }
                };
                let a78 = 1023u64;
                nat_add(a77, a78)?
            };
            let a80 = 1024u64;
            let a81 = 0u64;
            nat_quot(a79, a80, a81)
        }
    };
    let v6: Bytes = {
        let m83 = match v0.clone() {
            Adt2::C0(_, _, _, _, _, _, h82, _, _, _, _) => {
                h82
            }
        };
        match m83 {
            None => {
                f34(0u64, v0.clone(), Bytes::lit(&[0x00]))
            }
            Some(v5) => {
                let a93 = Bytes::lit(&[0x03]);
                let a94 = {
                    let a91 = f33(0u64);
                    let a92 = {
                        let a89 = f33(v4);
                        let a90 = {
                            let a87 = v5;
                            let a88 = {
                                let a85 = match v0.clone() {
                                    Adt2::C0(_, _, _, _, _, _, _, h84, _, _, _) => {
                                        h84
                                    }
                                };
                                let a86 = Bytes::lit(&[0x00]);
                                append_bytes(a85, a86)
                            };
                            append_bytes(a87, a88)
                        };
                        append_bytes(a89, a90)
                    };
                    append_bytes(a91, a92)
                };
                append_bytes(a93, a94)
            }
        }
    };
    let v7: (Adt2, Bytes) = f35(v0.clone(), v6, Bytes::lit(&[]))?;
    let v8: (Adt2, Bytes) = f35({
        let (h95, _) = v7;
        h95
    }, {
        let a100 = Bytes::lit(&[0x05]);
        let a101 = {
            let a98 = f33(match v0.clone() {
                Adt2::C0(_, _, _, _, _, _, _, _, _, _, h96) => {
                    h96
                }
            });
            let a99 = f36(match v0.clone() {
                Adt2::C0(_, _, _, _, _, _, _, _, _, h97, _) => {
                    h97
                }
            });
            append_bytes(a98, a99)
        };
        append_bytes(a100, a101)
    }, Bytes::lit(&[]))?;
    let v9: u64 = match v1.clone() {
        Adt8::C0(h102, _, _, _, _, _, _) => {
            h102
        }
    };
    f35({
        let (h103, _) = v8;
        h103
    }, f2(List::cons(Bytes::lit(&[0x06, 0x01]), List::cons(f37(v9), List::cons(f33(match v1.clone() {
        Adt8::C0(_, h104, _, _, _, _, _) => {
            h104
        }
    }), List::cons(f33(match v1.clone() {
        Adt8::C0(_, _, h105, _, _, _, _) => {
            h105
        }
    }), List::cons(v2, List::cons({
        let (_, h106) = v7;
        h106
    }, List::cons({
        let (_, h107) = v8;
        h107
    }, List::cons({
        let a108 = v9;
        let a109 = 2u64;
        if nat_eq(a108, a109) {
            v3
        } else {
            Bytes::lit(&[])
        }
    }, List::cons(f33({
        let a111 = match v0.clone() {
            Adt2::C0(_, _, _, _, _, _, _, _, h110, _, _) => {
                h110
            }
        };
        length_list(a111)
    }), List::cons(f36(match v0 {
        Adt2::C0(_, _, _, _, _, _, _, _, h112, _, _) => {
            h112
        }
    }), List::cons(f38(v1), List::<Bytes>::nil())))))))))))), Bytes::lit(&[]))
}

pub fn f16(v0: Fn2, p116: Adt2, p117: List<Adt3>) -> R<Adt2> {
    let m115 = (p116, p117);
    {
        let mut m115 = m115;
        loop {
            let step = {
                let (v1, v2) = m115;
                let m113 = v2;
                match m113.uncons() {
                    None => {
                        Err::<(Adt2, List<Adt3>), R<Adt2>>(Ok(v1))
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt2, List<Adt3>), R<Adt2>>(({
                            let c114: Fn2 = v0.clone();
                            c114.apply(v1, v3)?
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m115 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f17(v0: Adt2, v1: Adt3) -> R<Adt2> {
    f39(v0, v1)
}

pub fn f18(v0: Bytes, v1: u64) -> R<Bytes> {
    Ok(f38(f12(f32(f21(f6(), 0u32), v0)?, v1)?))
}

pub fn f19(v0: Bytes) -> R<Bytes> {
    let v1: Adt11 = f40(v0)?;
    let a138 = f41(match v1.clone() {
        Adt11::C0(h118, _, _, _, _, _, _, _) => {
            h118
        }
    });
    let a139 = {
        let a136 = f41(match v1.clone() {
            Adt11::C0(_, h119, _, _, _, _, _, _) => {
                h119
            }
        });
        let a137 = {
            let a134 = f41(match v1.clone() {
                Adt11::C0(_, _, h120, _, _, _, _, _) => {
                    h120
                }
            });
            let a135 = {
                let a132 = f41(match v1.clone() {
                    Adt11::C0(_, _, _, h121, _, _, _, _) => {
                        h121
                    }
                });
                let a133 = {
                    let a130 = f41(match v1.clone() {
                        Adt11::C0(_, _, _, _, h122, _, _, _) => {
                            h122
                        }
                    });
                    let a131 = {
                        let a128 = f41(match v1.clone() {
                            Adt11::C0(_, _, _, _, _, h123, _, _) => {
                                h123
                            }
                        });
                        let a129 = {
                            let a126 = f41(match v1.clone() {
                                Adt11::C0(_, _, _, _, _, _, h124, _) => {
                                    h124
                                }
                            });
                            let a127 = f41(match v1 {
                                Adt11::C0(_, _, _, _, _, _, _, h125) => {
                                    h125
                                }
                            });
                            append_bytes(a126, a127)
                        };
                        append_bytes(a128, a129)
                    };
                    append_bytes(a130, a131)
                };
                append_bytes(a132, a133)
            };
            append_bytes(a134, a135)
        };
        append_bytes(a136, a137)
    };
    Ok(append_bytes(a138, a139))
}

pub fn f20(v0: List<Bytes>) -> List<Bytes> {
    let v1: (Option<Bytes>, List<Bytes>) = f42(Fn5::F43, (None::<Bytes>, List::<Bytes>::nil()), v0);
    let v3: List<Bytes> = {
        let m141 = {
            let (h140, _) = v1;
            h140
        };
        match m141 {
            None => {
                let (_, h143) = v1;
                h143
            }
            Some(v2) => {
                List::cons(v2, {
                    let (_, h142) = v1;
                    h142
                })
            }
        }
    };
    f44(Fn6::F45, List::<Bytes>::nil(), v3)
}

pub fn f21(v0: Adt5, v1: u32) -> Adt9 {
    Adt9::C0(v0, v1, 0u64, Bytes::lit(&[]), List::<Adt7>::nil(), List::<Adt3>::nil(), List::<Adt3>::nil())
}

pub fn f22(v0: Bytes) -> R<Adt5> {
    Ok(f46(f47(v0.clone(), {
        let a144 = v0;
        length_bytes(a144)
    }, 0u64)?))
}

pub fn f23() -> u32 {
    16u32
}

pub fn f24(v0: Bytes) -> R<Adt9> {
    let v1: Adt8 = f48(v0)?;
    Ok(Adt9::C0(f46(f49(v1.clone(), 0u64)), f50(), 0u64, Bytes::lit(&[]), List::<Adt7>::nil(), List::<Adt3>::nil(), match v1 {
        Adt8::C0(_, _, _, _, _, _, h145) => {
            h145
        }
    }))
}

pub fn f25(v0: Adt5, v1: u32, v2: Bytes, v3: u64, v4: u64, v5: u64, v6: List<Adt3>) -> R<(Adt10, List<Adt3>)> {
    let v7: u64 = {
        let a146 = v3;
        let a147 = v4;
        nat_add(a146, a147)?
    };
    let v8: u64 = f51(v4)?;
    let v9: (u64, (Adt5, List<Adt3>)) = f52(Fn7::F53(v1, v3, v5, v0.clone(), v7, v2.clone()), {
        let a148 = v8;
        let a149 = 64u64;
        let a150 = 0u64;
        nat_quot(a148, a149, a150)
    }, (0u64, (v0.clone(), v6)))?;
    Ok((f54(v0, v1, v2, v7, v3, v5, v8, {
        let (h151, _) = {
            let (_, h152) = v9.clone();
            h152
        };
        h151
    }, {
        let a153 = v4;
        let a154 = v8;
        nat_sub(a153, a154)
    }, {
        let a155 = f55();
        let a156 = f56(v8);
        fixed_u32::bit_or(a155, a156)
    })?, {
        let (_, h157) = {
            let (_, h158) = v9;
            h158
        };
        h157
    }))
}

pub fn f26(v0: Fn3, p162: (Adt10, (List<Adt3>, u64)), p163: List<Adt7>) -> R<(Adt10, (List<Adt3>, u64))> {
    let m161 = (p162, p163);
    {
        let mut m161 = m161;
        loop {
            let step = {
                let (v1, v2) = m161;
                let m159 = v2;
                match m159.uncons() {
                    None => {
                        Err::<((Adt10, (List<Adt3>, u64)), List<Adt7>), R<(Adt10, (List<Adt3>, u64))>>(Ok(v1))
                    }
                    Some((v3, v4)) => {
                        Ok::<((Adt10, (List<Adt3>, u64)), List<Adt7>), R<(Adt10, (List<Adt3>, u64))>>(({
                            let c160: Fn3 = v0.clone();
                            c160.apply(v1, v3)?
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m161 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f27(v0: u32, v1: Adt5, v2: (Adt10, (List<Adt3>, u64)), v3: Adt7) -> R<(Adt10, (List<Adt3>, u64))> {
    let v4: Adt4 = f57({
        let (h164, _) = v2.clone();
        h164
    })?;
    Ok((f58(v3.clone(), Adt7::C0(f46(match v4.clone() {
        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h165) => {
            h165
        }
    }), {
        let m168 = {
            let (h167, _) = v2.clone();
            h167
        };
        match m168 {
            Adt10::C0(_, h166, _, _, _, _, _, _, _, _) => {
                h166
            }
        }
    }, {
        let m171 = {
            let (h170, _) = v2.clone();
            h170
        };
        match m171 {
            Adt10::C0(_, _, h169, _, _, _, _, _, _, _) => {
                h169
            }
        }
    }), v1, v0)?, (List::cons(Adt3::C2(v3, {
        let (_, h172) = {
            let (_, h173) = v2.clone();
            h173
        };
        h172
    }), List::cons(Adt3::C0(v4), {
        let (h174, _) = {
            let (_, h175) = v2.clone();
            h175
        };
        h174
    })), {
        let a178 = {
            let (_, h176) = {
                let (_, h177) = v2;
                h177
            };
            h176
        };
        let a179 = 1u64;
        nat_sub(a178, a179)
    })))
}

pub fn f28(v0: Fn4, p183: u64, p184: (u64, List<Adt3>)) -> R<(u64, List<Adt3>)> {
    let m182 = (p183, p184);
    {
        let mut m182 = m182;
        loop {
            let step = {
                let (v1, v2) = m182;
                let m180 = v1;
                if m180 == 0 {
                    Err::<(u64, (u64, List<Adt3>)), R<(u64, List<Adt3>)>>(Ok(v2))
                } else {
                    let v3 = m180 - 1;
                    Ok::<(u64, (u64, List<Adt3>)), R<(u64, List<Adt3>)>>((v3, {
                        let c181: Fn4 = v0.clone();
                        c181.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m182 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f29(v0: (Adt10, (List<Adt3>, u64)), v1: (u64, List<Adt3>)) -> R<(u64, List<Adt3>)> {
    Ok(({
        let a186 = {
            let (h185, _) = v1.clone();
            h185
        };
        let a187 = 1u64;
        nat_add(a186, a187)?
    }, List::cons(Adt3::C0(f57(f59({
        let (h188, _) = v0;
        h188
    }, {
        let (h189, _) = v1;
        h189
    }))?), {
        let (_, h190) = v1;
        h190
    })))
}

pub fn f30(v0: u32) -> u64 {
    let a191 = v0;
    let a192 = f23();
    if equal(a191, a192) {
        1u64
    } else {
        let a193 = v0;
        let a194 = f50();
        if equal(a193, a194) {
            2u64
        } else {
            let a195 = v0;
            let a196 = f7();
            if equal(a195, a196) {
                3u64
            } else {
                0u64
            }
        }
    }
}

pub fn f31(v0: List<Adt3>) -> List<Adt3> {
    f60(Fn8::F61, List::<Adt3>::nil(), v0)
}

pub fn f32(v0: Adt9, v1: Bytes) -> R<Adt9> {
    let v2: Bytes = {
        let a198 = match v0.clone() {
            Adt9::C0(_, _, _, h197, _, _, _) => {
                h197
            }
        };
        let a199 = v1;
        append_bytes(a198, a199)
    };
    let v3: u64 = {
        let a200 = v2.clone();
        length_bytes(a200)
    };
    let v4: u64 = {
        let a201 = v3;
        let a202 = 0u64;
        if nat_eq(a201, a202) {
            0u64
        } else {
            let a205 = {
                let a203 = v3;
                let a204 = 1u64;
                nat_sub(a203, a204)
            };
            let a206 = 1024u64;
            let a207 = 0u64;
            nat_quot(a205, a206, a207)
        }
    };
    let v5: u32 = match v0.clone() {
        Adt9::C0(_, h208, _, _, _, _, _) => {
            h208
        }
    };
    let v6: Adt5 = match v0.clone() {
        Adt9::C0(h209, _, _, _, _, _, _) => {
            h209
        }
    };
    let v7: (u64, (List<Adt7>, List<Adt3>)) = f62(Fn9::F63(v5, v2.clone(), v6.clone(), v0.clone()), v4, (0u64, (match v0.clone() {
        Adt9::C0(_, _, _, _, h210, _, _) => {
            h210
        }
    }, match v0.clone() {
        Adt9::C0(_, _, _, _, _, h211, _) => {
            h211
        }
    })))?;
    let v8: u64 = {
        let a212 = v4;
        let a213 = 1024u64;
        nat_mul(a212, a213)?
    };
    Ok(Adt9::C0(v6, v5, {
        let a215 = match v0.clone() {
            Adt9::C0(_, _, h214, _, _, _, _) => {
                h214
            }
        };
        let a216 = v4;
        nat_add(a215, a216)?
    }, {
        let m222 = {
            let a219 = v2;
            let a220 = v8;
            let a221 = {
                let a217 = v3;
                let a218 = v8;
                nat_sub(a217, a218)
            };
            slice_bytes(a219, a220, a221)
        };
        match m222 {
            None => {
                Bytes::lit(&[])
            }
            Some(v9) => {
                v9
            }
        }
    }, {
        let (h223, _) = {
            let (_, h224) = v7.clone();
            h224
        };
        h223
    }, {
        let (_, h225) = {
            let (_, h226) = v7;
            h226
        };
        h225
    }, match v0 {
        Adt9::C0(_, _, _, _, _, _, h227) => {
            h227
        }
    }))
}

pub fn f33(v0: u64) -> Bytes {
    let a228 = f64(f65(v0));
    let a229 = f64(f66(v0));
    append_bytes(a228, a229)
}

pub fn f34(v0: u64, v1: Adt2, v2: Bytes) -> Bytes {
    let a238 = Bytes::lit(&[0x02]);
    let a239 = {
        let a236 = f33(v0);
        let a237 = {
            let a234 = f37(match v1.clone() {
                Adt2::C0(_, _, _, _, _, h230, _, _, _, _, _) => {
                    h230
                }
            });
            let a235 = {
                let a232 = f36(match v1 {
                    Adt2::C0(_, _, _, _, h231, _, _, _, _, _, _) => {
                        h231
                    }
                });
                let a233 = v2;
                append_bytes(a232, a233)
            };
            append_bytes(a234, a235)
        };
        append_bytes(a236, a237)
    };
    append_bytes(a238, a239)
}

pub fn f35(v0: Adt2, v1: Bytes, v2: Bytes) -> R<(Adt2, Bytes)> {
    let v3: Bytes = f8(v1.clone())?;
    Ok((Adt2::C0(match v0.clone() {
        Adt2::C0(h240, _, _, _, _, _, _, _, _, _, _) => {
            h240
        }
    }, List::cons({
        let a241 = f67(v1, v3.clone());
        let a242 = v2;
        append_bytes(a241, a242)
    }, match v0.clone() {
        Adt2::C0(_, h243, _, _, _, _, _, _, _, _, _) => {
            h243
        }
    }), {
        let a245 = match v0.clone() {
            Adt2::C0(_, _, h244, _, _, _, _, _, _, _, _) => {
                h244
            }
        };
        let a246 = 1u64;
        nat_add(a245, a246)?
    }, match v0.clone() {
        Adt2::C0(_, _, _, h247, _, _, _, _, _, _, _) => {
            h247
        }
    }, match v0.clone() {
        Adt2::C0(_, _, _, _, h248, _, _, _, _, _, _) => {
            h248
        }
    }, match v0.clone() {
        Adt2::C0(_, _, _, _, _, h249, _, _, _, _, _) => {
            h249
        }
    }, match v0.clone() {
        Adt2::C0(_, _, _, _, _, _, h250, _, _, _, _) => {
            h250
        }
    }, match v0.clone() {
        Adt2::C0(_, _, _, _, _, _, _, h251, _, _, _) => {
            h251
        }
    }, match v0.clone() {
        Adt2::C0(_, _, _, _, _, _, _, _, h252, _, _) => {
            h252
        }
    }, match v0.clone() {
        Adt2::C0(_, _, _, _, _, _, _, _, _, h253, _) => {
            h253
        }
    }, match v0 {
        Adt2::C0(_, _, _, _, _, _, _, _, _, _, h254) => {
            h254
        }
    }), f68(v3)))
}

pub fn f36(v0: List<Bytes>) -> Bytes {
    f2(f44(Fn6::F69, List::<Bytes>::nil(), v0))
}

pub fn f37(v0: u64) -> Bytes {
    f70(f71(v0, 8u64))
}

pub fn f38(v0: Adt8) -> Bytes {
    let v1: List<Bytes> = f72(Fn10::F73, List::<Bytes>::nil(), match v0.clone() {
        Adt8::C0(_, _, _, _, _, _, h255) => {
            h255
        }
    });
    let v2: Bytes = f2(f44(Fn6::F74, List::<Bytes>::nil(), v1));
    let m260 = {
        let a257 = v2.clone();
        let a258 = 0u64;
        let a259 = match v0 {
            Adt8::C0(_, _, h256, _, _, _, _) => {
                h256
            }
        };
        slice_bytes(a257, a258, a259)
    };
    match m260 {
        None => {
            v2
        }
        Some(v3) => {
            v3
        }
    }
}

pub fn f39(v0: Adt2, v1: Adt3) -> R<Adt2> {
    let m261 = v1;
    match m261 {
        Adt3::C0(v2) => {
            let v3: Bytes = f75(v2.clone());
            let v4: (Adt2, Bytes) = f35(v0.clone(), v3, {
                let a263 = match v0.clone() {
                    Adt2::C0(h262, _, _, _, _, _, _, _, _, _, _) => {
                        h262
                    }
                };
                let a264 = 0u64;
                if nat_eq(a263, a264) {
                    Bytes::lit(&[])
                } else {
                    f76(match v0 {
                        Adt2::C0(h265, _, _, _, _, _, _, _, _, _, _) => {
                            h265
                        }
                    }, v2.clone())?
                }
            })?;
            let v5: Adt2 = {
                let (h266, _) = v4;
                h266
            };
            let v6: Bytes = {
                let (_, h267) = v4;
                h267
            };
            let v7: List<Bytes> = List::cons({
                let a268 = Bytes::lit(&[0x00]);
                let a269 = v6.clone();
                append_bytes(a268, a269)
            }, match v5.clone() {
                Adt2::C0(_, _, _, _, _, _, _, _, _, h270, _) => {
                    h270
                }
            });
            let a272 = match v2.clone() {
                Adt4::C0(h271, _, _, _, _, _, _, _, _, _, _, _) => {
                    h271
                }
            };
            let a273 = 0u64;
            if nat_eq(a272, a273) {
                let a277 = {
                    let a275 = match v2.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h274, _) => {
                            h274
                        }
                    };
                    let a276 = f55();
                    fixed_u32::bit_and(a275, a276)
                };
                let a278 = f55();
                if equal(a277, a278) {
                    let v8: (Adt2, Bytes) = f35(v5.clone(), f34(match v2.clone() {
                        Adt4::C0(_, h279, _, _, _, _, _, _, _, _, _, _) => {
                            h279
                        }
                    }, Adt2::C0(match v5.clone() {
                        Adt2::C0(h280, _, _, _, _, _, _, _, _, _, _) => {
                            h280
                        }
                    }, match v5.clone() {
                        Adt2::C0(_, h281, _, _, _, _, _, _, _, _, _) => {
                            h281
                        }
                    }, match v5.clone() {
                        Adt2::C0(_, _, h282, _, _, _, _, _, _, _, _) => {
                            h282
                        }
                    }, match v5.clone() {
                        Adt2::C0(_, _, _, h283, _, _, _, _, _, _, _) => {
                            h283
                        }
                    }, List::cons(v6, match v5.clone() {
                        Adt2::C0(_, _, _, _, h284, _, _, _, _, _, _) => {
                            h284
                        }
                    }), {
                        let a286 = match v5.clone() {
                            Adt2::C0(_, _, _, _, _, h285, _, _, _, _, _) => {
                                h285
                            }
                        };
                        let a287 = 1u64;
                        nat_add(a286, a287)?
                    }, match v5.clone() {
                        Adt2::C0(_, _, _, _, _, _, h288, _, _, _, _) => {
                            h288
                        }
                    }, match v5.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, h289, _, _, _) => {
                            h289
                        }
                    }, match v5.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, _, h290, _, _) => {
                            h290
                        }
                    }, match v5.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, _, _, h291, _) => {
                            h291
                        }
                    }, match v5.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, _, _, _, h292) => {
                            h292
                        }
                    }), {
                        let a294 = Bytes::lit(&[0x01]);
                        let a295 = f77(f46(match v2 {
                            Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h293) => {
                                h293
                            }
                        }));
                        append_bytes(a294, a295)
                    }), Bytes::lit(&[]))?;
                    let v9: Adt2 = {
                        let (h296, _) = v8;
                        h296
                    };
                    Ok(Adt2::C0(match v9.clone() {
                        Adt2::C0(h297, _, _, _, _, _, _, _, _, _, _) => {
                            h297
                        }
                    }, match v9.clone() {
                        Adt2::C0(_, h298, _, _, _, _, _, _, _, _, _) => {
                            h298
                        }
                    }, match v9.clone() {
                        Adt2::C0(_, _, h299, _, _, _, _, _, _, _, _) => {
                            h299
                        }
                    }, match v9.clone() {
                        Adt2::C0(_, _, _, h300, _, _, _, _, _, _, _) => {
                            h300
                        }
                    }, List::<Bytes>::nil(), 0u64, match v9.clone() {
                        Adt2::C0(_, _, _, _, _, _, h301, _, _, _, _) => {
                            h301
                        }
                    }, {
                        let (_, h302) = v8;
                        h302
                    }, match v9 {
                        Adt2::C0(_, _, _, _, _, _, _, _, h303, _, _) => {
                            h303
                        }
                    }, v7, {
                        let a305 = match v5 {
                            Adt2::C0(_, _, _, _, _, _, _, _, _, _, h304) => {
                                h304
                            }
                        };
                        let a306 = 1u64;
                        nat_add(a305, a306)?
                    }))
                } else {
                    Ok(Adt2::C0(match v5.clone() {
                        Adt2::C0(h307, _, _, _, _, _, _, _, _, _, _) => {
                            h307
                        }
                    }, match v5.clone() {
                        Adt2::C0(_, h308, _, _, _, _, _, _, _, _, _) => {
                            h308
                        }
                    }, match v5.clone() {
                        Adt2::C0(_, _, h309, _, _, _, _, _, _, _, _) => {
                            h309
                        }
                    }, match v5.clone() {
                        Adt2::C0(_, _, _, h310, _, _, _, _, _, _, _) => {
                            h310
                        }
                    }, List::cons(v6, match v5.clone() {
                        Adt2::C0(_, _, _, _, h311, _, _, _, _, _, _) => {
                            h311
                        }
                    }), {
                        let a313 = match v5.clone() {
                            Adt2::C0(_, _, _, _, _, h312, _, _, _, _, _) => {
                                h312
                            }
                        };
                        let a314 = 1u64;
                        nat_add(a313, a314)?
                    }, match v5.clone() {
                        Adt2::C0(_, _, _, _, _, _, h315, _, _, _, _) => {
                            h315
                        }
                    }, match v5.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, h316, _, _, _) => {
                            h316
                        }
                    }, match v5.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, _, h317, _, _) => {
                            h317
                        }
                    }, v7, {
                        let a319 = match v5 {
                            Adt2::C0(_, _, _, _, _, _, _, _, _, _, h318) => {
                                h318
                            }
                        };
                        let a320 = 1u64;
                        nat_add(a319, a320)?
                    }))
                }
            } else {
                let a322 = match v2.clone() {
                    Adt4::C0(h321, _, _, _, _, _, _, _, _, _, _, _) => {
                        h321
                    }
                };
                let a323 = 1u64;
                if nat_eq(a322, a323) {
                    let m325 = match v5.clone() {
                        Adt2::C0(_, _, _, _, _, _, h324, _, _, _, _) => {
                            h324
                        }
                    };
                    match m325 {
                        None => {
                            Ok(Adt2::C0(match v5.clone() {
                                Adt2::C0(h356, _, _, _, _, _, _, _, _, _, _) => {
                                    h356
                                }
                            }, match v5.clone() {
                                Adt2::C0(_, h357, _, _, _, _, _, _, _, _, _) => {
                                    h357
                                }
                            }, match v5.clone() {
                                Adt2::C0(_, _, h358, _, _, _, _, _, _, _, _) => {
                                    h358
                                }
                            }, match v5.clone() {
                                Adt2::C0(_, _, _, h359, _, _, _, _, _, _, _) => {
                                    h359
                                }
                            }, match v5.clone() {
                                Adt2::C0(_, _, _, _, h360, _, _, _, _, _, _) => {
                                    h360
                                }
                            }, match v5.clone() {
                                Adt2::C0(_, _, _, _, _, h361, _, _, _, _, _) => {
                                    h361
                                }
                            }, match v5.clone() {
                                Adt2::C0(_, _, _, _, _, _, h362, _, _, _, _) => {
                                    h362
                                }
                            }, match v5.clone() {
                                Adt2::C0(_, _, _, _, _, _, _, h363, _, _, _) => {
                                    h363
                                }
                            }, match v5.clone() {
                                Adt2::C0(_, _, _, _, _, _, _, _, h364, _, _) => {
                                    h364
                                }
                            }, v7, {
                                let a366 = match v5 {
                                    Adt2::C0(_, _, _, _, _, _, _, _, _, _, h365) => {
                                        h365
                                    }
                                };
                                let a367 = 1u64;
                                nat_add(a366, a367)?
                            }))
                        }
                        Some(v10) => {
                            let v11: (Adt2, Bytes) = f35(v5.clone(), {
                                let a342 = Bytes::lit(&[0x03]);
                                let a343 = {
                                    let a340 = f33(match v2.clone() {
                                        Adt4::C0(_, h326, _, _, _, _, _, _, _, _, _, _) => {
                                            h326
                                        }
                                    });
                                    let a341 = {
                                        let a338 = f33(match v2.clone() {
                                            Adt4::C0(_, _, h327, _, _, _, _, _, _, _, _, _) => {
                                                h327
                                            }
                                        });
                                        let a339 = {
                                            let a336 = v10;
                                            let a337 = {
                                                let a334 = match v5.clone() {
                                                    Adt2::C0(_, _, _, _, _, _, _, h328, _, _, _) => {
                                                        h328
                                                    }
                                                };
                                                let a335 = {
                                                    let a332 = Bytes::lit(&[0x01]);
                                                    let a333 = {
                                                        let a330 = v6;
                                                        let a331 = f77(f46(match v2 {
                                                            Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h329) => {
                                                                h329
                                                            }
                                                        }));
                                                        append_bytes(a330, a331)
                                                    };
                                                    append_bytes(a332, a333)
                                                };
                                                append_bytes(a334, a335)
                                            };
                                            append_bytes(a336, a337)
                                        };
                                        append_bytes(a338, a339)
                                    };
                                    append_bytes(a340, a341)
                                };
                                append_bytes(a342, a343)
                            }, Bytes::lit(&[]))?;
                            let v12: Adt2 = {
                                let (h344, _) = v11;
                                h344
                            };
                            Ok(Adt2::C0(match v12.clone() {
                                Adt2::C0(h345, _, _, _, _, _, _, _, _, _, _) => {
                                    h345
                                }
                            }, match v12.clone() {
                                Adt2::C0(_, h346, _, _, _, _, _, _, _, _, _) => {
                                    h346
                                }
                            }, match v12.clone() {
                                Adt2::C0(_, _, h347, _, _, _, _, _, _, _, _) => {
                                    h347
                                }
                            }, match v12.clone() {
                                Adt2::C0(_, _, _, h348, _, _, _, _, _, _, _) => {
                                    h348
                                }
                            }, match v12.clone() {
                                Adt2::C0(_, _, _, _, h349, _, _, _, _, _, _) => {
                                    h349
                                }
                            }, match v12.clone() {
                                Adt2::C0(_, _, _, _, _, h350, _, _, _, _, _) => {
                                    h350
                                }
                            }, None::<Bytes>, {
                                let (_, h351) = v11;
                                h351
                            }, match v12 {
                                Adt2::C0(_, _, _, _, _, _, _, _, h352, _, _) => {
                                    h352
                                }
                            }, v7, {
                                let a354 = match v5 {
                                    Adt2::C0(_, _, _, _, _, _, _, _, _, _, h353) => {
                                        h353
                                    }
                                };
                                let a355 = 1u64;
                                nat_add(a354, a355)?
                            }))
                        }
                    }
                } else {
                    let v13: (Adt2, Bytes) = f35(v5.clone(), {
                        let a374 = Bytes::lit(&[0x04]);
                        let a375 = {
                            let a372 = f33(match v2.clone() {
                                Adt4::C0(_, _, _, h368, _, _, _, _, _, _, _, _) => {
                                    h368
                                }
                            });
                            let a373 = {
                                let a370 = v6;
                                let a371 = f78(match v2 {
                                    Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h369) => {
                                        h369
                                    }
                                });
                                append_bytes(a370, a371)
                            };
                            append_bytes(a372, a373)
                        };
                        append_bytes(a374, a375)
                    }, Bytes::lit(&[]))?;
                    let v14: Adt2 = {
                        let (h376, _) = v13;
                        h376
                    };
                    Ok(Adt2::C0(match v14.clone() {
                        Adt2::C0(h377, _, _, _, _, _, _, _, _, _, _) => {
                            h377
                        }
                    }, match v14.clone() {
                        Adt2::C0(_, h378, _, _, _, _, _, _, _, _, _) => {
                            h378
                        }
                    }, match v14.clone() {
                        Adt2::C0(_, _, h379, _, _, _, _, _, _, _, _) => {
                            h379
                        }
                    }, match v14.clone() {
                        Adt2::C0(_, _, _, h380, _, _, _, _, _, _, _) => {
                            h380
                        }
                    }, match v14.clone() {
                        Adt2::C0(_, _, _, _, h381, _, _, _, _, _, _) => {
                            h381
                        }
                    }, match v14.clone() {
                        Adt2::C0(_, _, _, _, _, h382, _, _, _, _, _) => {
                            h382
                        }
                    }, match v14.clone() {
                        Adt2::C0(_, _, _, _, _, _, h383, _, _, _, _) => {
                            h383
                        }
                    }, match v14.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, h384, _, _, _) => {
                            h384
                        }
                    }, List::cons({
                        let (_, h385) = v13;
                        h385
                    }, match v14 {
                        Adt2::C0(_, _, _, _, _, _, _, _, h386, _, _) => {
                            h386
                        }
                    }), v7, {
                        let a388 = match v5 {
                            Adt2::C0(_, _, _, _, _, _, _, _, _, _, h387) => {
                                h387
                            }
                        };
                        let a389 = 1u64;
                        nat_add(a388, a389)?
                    }))
                }
            }
        }
        Adt3::C1(v15, v16) => {
            Ok(Adt2::C0(match v0.clone() {
                Adt2::C0(h390, _, _, _, _, _, _, _, _, _, _) => {
                    h390
                }
            }, match v0.clone() {
                Adt2::C0(_, h391, _, _, _, _, _, _, _, _, _) => {
                    h391
                }
            }, match v0.clone() {
                Adt2::C0(_, _, h392, _, _, _, _, _, _, _, _) => {
                    h392
                }
            }, List::cons(match v0.clone() {
                Adt2::C0(_, _, _, _, _, _, _, h393, _, _, _) => {
                    h393
                }
            }, match v0.clone() {
                Adt2::C0(_, _, _, h394, _, _, _, _, _, _, _) => {
                    h394
                }
            }), match v0.clone() {
                Adt2::C0(_, _, _, _, h395, _, _, _, _, _, _) => {
                    h395
                }
            }, match v0.clone() {
                Adt2::C0(_, _, _, _, _, h396, _, _, _, _, _) => {
                    h396
                }
            }, match v0.clone() {
                Adt2::C0(_, _, _, _, _, _, h397, _, _, _, _) => {
                    h397
                }
            }, match v0.clone() {
                Adt2::C0(_, _, _, _, _, _, _, h398, _, _, _) => {
                    h398
                }
            }, match v0.clone() {
                Adt2::C0(_, _, _, _, _, _, _, _, h399, _, _) => {
                    h399
                }
            }, List::cons({
                let a409 = Bytes::lit(&[0x01]);
                let a410 = {
                    let a407 = f37(v16);
                    let a408 = {
                        let a405 = f33(match v15.clone() {
                            Adt7::C0(_, h400, _) => {
                                h400
                            }
                        });
                        let a406 = {
                            let a403 = f33(match v15.clone() {
                                Adt7::C0(_, _, h401) => {
                                    h401
                                }
                            });
                            let a404 = f77(match v15 {
                                Adt7::C0(h402, _, _) => {
                                    h402
                                }
                            });
                            append_bytes(a403, a404)
                        };
                        append_bytes(a405, a406)
                    };
                    append_bytes(a407, a408)
                };
                append_bytes(a409, a410)
            }, match v0.clone() {
                Adt2::C0(_, _, _, _, _, _, _, _, _, h411, _) => {
                    h411
                }
            }), {
                let a413 = match v0 {
                    Adt2::C0(_, _, _, _, _, _, _, _, _, _, h412) => {
                        h412
                    }
                };
                let a414 = 1u64;
                nat_add(a413, a414)?
            }))
        }
        Adt3::C2(v17, v18) => {
            let m416 = match v0.clone() {
                Adt2::C0(_, _, _, h415, _, _, _, _, _, _, _) => {
                    h415
                }
            };
            match m416.uncons() {
                None => {
                    Ok(Adt2::C0(match v0.clone() {
                        Adt2::C0(h439, _, _, _, _, _, _, _, _, _, _) => {
                            h439
                        }
                    }, match v0.clone() {
                        Adt2::C0(_, h440, _, _, _, _, _, _, _, _, _) => {
                            h440
                        }
                    }, match v0.clone() {
                        Adt2::C0(_, _, h441, _, _, _, _, _, _, _, _) => {
                            h441
                        }
                    }, match v0.clone() {
                        Adt2::C0(_, _, _, h442, _, _, _, _, _, _, _) => {
                            h442
                        }
                    }, match v0.clone() {
                        Adt2::C0(_, _, _, _, h443, _, _, _, _, _, _) => {
                            h443
                        }
                    }, match v0.clone() {
                        Adt2::C0(_, _, _, _, _, h444, _, _, _, _, _) => {
                            h444
                        }
                    }, match v0.clone() {
                        Adt2::C0(_, _, _, _, _, _, h445, _, _, _, _) => {
                            h445
                        }
                    }, match v0.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, h446, _, _, _) => {
                            h446
                        }
                    }, match v0.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, _, h447, _, _) => {
                            h447
                        }
                    }, List::cons({
                        let a457 = Bytes::lit(&[0x02]);
                        let a458 = {
                            let a455 = f37(v18);
                            let a456 = {
                                let a453 = f33(match v17.clone() {
                                    Adt7::C0(_, h448, _) => {
                                        h448
                                    }
                                });
                                let a454 = {
                                    let a451 = f33(match v17.clone() {
                                        Adt7::C0(_, _, h449) => {
                                            h449
                                        }
                                    });
                                    let a452 = f77(match v17 {
                                        Adt7::C0(h450, _, _) => {
                                            h450
                                        }
                                    });
                                    append_bytes(a451, a452)
                                };
                                append_bytes(a453, a454)
                            };
                            append_bytes(a455, a456)
                        };
                        append_bytes(a457, a458)
                    }, match v0.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, _, _, h459, _) => {
                            h459
                        }
                    }), {
                        let a461 = match v0 {
                            Adt2::C0(_, _, _, _, _, _, _, _, _, _, h460) => {
                                h460
                            }
                        };
                        let a462 = 1u64;
                        nat_add(a461, a462)?
                    }))
                }
                Some((v19, v20)) => {
                    Ok(Adt2::C0(match v0.clone() {
                        Adt2::C0(h417, _, _, _, _, _, _, _, _, _, _) => {
                            h417
                        }
                    }, match v0.clone() {
                        Adt2::C0(_, h418, _, _, _, _, _, _, _, _, _) => {
                            h418
                        }
                    }, match v0.clone() {
                        Adt2::C0(_, _, h419, _, _, _, _, _, _, _, _) => {
                            h419
                        }
                    }, v20, match v0.clone() {
                        Adt2::C0(_, _, _, _, h420, _, _, _, _, _, _) => {
                            h420
                        }
                    }, match v0.clone() {
                        Adt2::C0(_, _, _, _, _, h421, _, _, _, _, _) => {
                            h421
                        }
                    }, Some(v19), match v0.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, h422, _, _, _) => {
                            h422
                        }
                    }, match v0.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, _, h423, _, _) => {
                            h423
                        }
                    }, List::cons({
                        let a433 = Bytes::lit(&[0x02]);
                        let a434 = {
                            let a431 = f37(v18);
                            let a432 = {
                                let a429 = f33(match v17.clone() {
                                    Adt7::C0(_, h424, _) => {
                                        h424
                                    }
                                });
                                let a430 = {
                                    let a427 = f33(match v17.clone() {
                                        Adt7::C0(_, _, h425) => {
                                            h425
                                        }
                                    });
                                    let a428 = f77(match v17 {
                                        Adt7::C0(h426, _, _) => {
                                            h426
                                        }
                                    });
                                    append_bytes(a427, a428)
                                };
                                append_bytes(a429, a430)
                            };
                            append_bytes(a431, a432)
                        };
                        append_bytes(a433, a434)
                    }, match v0.clone() {
                        Adt2::C0(_, _, _, _, _, _, _, _, _, h435, _) => {
                            h435
                        }
                    }), {
                        let a437 = match v0 {
                            Adt2::C0(_, _, _, _, _, _, _, _, _, _, h436) => {
                                h436
                            }
                        };
                        let a438 = 1u64;
                        nat_add(a437, a438)?
                    }))
                }
            }
        }
    }
}

pub fn f40(v0: Bytes) -> R<Adt11> {
    let v1: u64 = {
        let a463 = v0.clone();
        length_bytes(a463)
    };
    let v2: u64 = {
        let a469 = {
            let a466 = {
                let a464 = v1;
                let a465 = 72u64;
                nat_add(a464, a465)?
            };
            let a467 = 64u64;
            let a468 = 0u64;
            nat_quot(a466, a467, a468)
        };
        let a470 = 64u64;
        nat_mul(a469, a470)?
    };
    let (_, h471) = f79(Fn11::F80(v0, v1, v2), {
        let a472 = v2;
        let a473 = 64u64;
        let a474 = 0u64;
        nat_quot(a472, a473, a474)
    }, (0u64, f81()))?;
    Ok(h471)
}

pub fn f41(v0: u32) -> Bytes {
    let a487 = f70({
        let a475 = f82(v0, 24u32);
        let a476 = 255u32;
        fixed_u32::bit_and(a475, a476)
    });
    let a488 = {
        let a485 = f70({
            let a477 = f82(v0, 16u32);
            let a478 = 255u32;
            fixed_u32::bit_and(a477, a478)
        });
        let a486 = {
            let a483 = f70({
                let a479 = f82(v0, 8u32);
                let a480 = 255u32;
                fixed_u32::bit_and(a479, a480)
            });
            let a484 = f70({
                let a481 = f82(v0, 0u32);
                let a482 = 255u32;
                fixed_u32::bit_and(a481, a482)
            });
            append_bytes(a483, a484)
        };
        append_bytes(a485, a486)
    };
    append_bytes(a487, a488)
}

pub fn f42(v0: Fn5, p492: (Option<Bytes>, List<Bytes>), p493: List<Bytes>) -> (Option<Bytes>, List<Bytes>) {
    let m491 = (p492, p493);
    {
        let mut m491 = m491;
        loop {
            let step = {
                let (v1, v2) = m491;
                let m489 = v2;
                match m489.uncons() {
                    None => {
                        Err::<((Option<Bytes>, List<Bytes>), List<Bytes>), (Option<Bytes>, List<Bytes>)>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<((Option<Bytes>, List<Bytes>), List<Bytes>), (Option<Bytes>, List<Bytes>)>(({
                            let c490: Fn5 = v0.clone();
                            c490.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m491 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f43(v0: (Option<Bytes>, List<Bytes>), v1: Bytes) -> (Option<Bytes>, List<Bytes>) {
    let m495 = {
        let (h494, _) = v0;
        h494
    };
    match m495 {
        None => {
            (Some(v1), {
                let (_, h499) = v0;
                h499
            })
        }
        Some(v2) => {
            (None::<Bytes>, List::cons({
                let a496 = v2;
                let a497 = v1;
                append_bytes(a496, a497)
            }, {
                let (_, h498) = v0;
                h498
            }))
        }
    }
}

pub fn f44(v0: Fn6, p503: List<Bytes>, p504: List<Bytes>) -> List<Bytes> {
    let m502 = (p503, p504);
    {
        let mut m502 = m502;
        loop {
            let step = {
                let (v1, v2) = m502;
                let m500 = v2;
                match m500.uncons() {
                    None => {
                        Err::<(List<Bytes>, List<Bytes>), List<Bytes>>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(List<Bytes>, List<Bytes>), List<Bytes>>(({
                            let c501: Fn6 = v0.clone();
                            c501.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m502 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f45(v0: List<Bytes>, v1: Bytes) -> List<Bytes> {
    List::cons(v1, v0)
}

pub fn f46(v0: Adt6) -> Adt5 {
    Adt5::C0(match v0.clone() {
        Adt6::C0(h505, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h505
        }
    }, match v0.clone() {
        Adt6::C0(_, h506, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h506
        }
    }, match v0.clone() {
        Adt6::C0(_, _, h507, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h507
        }
    }, match v0.clone() {
        Adt6::C0(_, _, _, h508, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h508
        }
    }, match v0.clone() {
        Adt6::C0(_, _, _, _, h509, _, _, _, _, _, _, _, _, _, _, _) => {
            h509
        }
    }, match v0.clone() {
        Adt6::C0(_, _, _, _, _, h510, _, _, _, _, _, _, _, _, _, _) => {
            h510
        }
    }, match v0.clone() {
        Adt6::C0(_, _, _, _, _, _, h511, _, _, _, _, _, _, _, _, _) => {
            h511
        }
    }, match v0 {
        Adt6::C0(_, _, _, _, _, _, _, h512, _, _, _, _, _, _, _, _) => {
            h512
        }
    })
}

pub fn f47(v0: Bytes, v1: u64, v2: u64) -> R<Adt6> {
    Ok(Adt6::C0(f83(v0.clone(), v1, {
        let a513 = v2;
        let a514 = 0u64;
        nat_add(a513, a514)?
    })?, f83(v0.clone(), v1, {
        let a515 = v2;
        let a516 = 4u64;
        nat_add(a515, a516)?
    })?, f83(v0.clone(), v1, {
        let a517 = v2;
        let a518 = 8u64;
        nat_add(a517, a518)?
    })?, f83(v0.clone(), v1, {
        let a519 = v2;
        let a520 = 12u64;
        nat_add(a519, a520)?
    })?, f83(v0.clone(), v1, {
        let a521 = v2;
        let a522 = 16u64;
        nat_add(a521, a522)?
    })?, f83(v0.clone(), v1, {
        let a523 = v2;
        let a524 = 20u64;
        nat_add(a523, a524)?
    })?, f83(v0.clone(), v1, {
        let a525 = v2;
        let a526 = 24u64;
        nat_add(a525, a526)?
    })?, f83(v0.clone(), v1, {
        let a527 = v2;
        let a528 = 28u64;
        nat_add(a527, a528)?
    })?, f83(v0.clone(), v1, {
        let a529 = v2;
        let a530 = 32u64;
        nat_add(a529, a530)?
    })?, f83(v0.clone(), v1, {
        let a531 = v2;
        let a532 = 36u64;
        nat_add(a531, a532)?
    })?, f83(v0.clone(), v1, {
        let a533 = v2;
        let a534 = 40u64;
        nat_add(a533, a534)?
    })?, f83(v0.clone(), v1, {
        let a535 = v2;
        let a536 = 44u64;
        nat_add(a535, a536)?
    })?, f83(v0.clone(), v1, {
        let a537 = v2;
        let a538 = 48u64;
        nat_add(a537, a538)?
    })?, f83(v0.clone(), v1, {
        let a539 = v2;
        let a540 = 52u64;
        nat_add(a539, a540)?
    })?, f83(v0.clone(), v1, {
        let a541 = v2;
        let a542 = 56u64;
        nat_add(a541, a542)?
    })?, f83(v0, v1, {
        let a543 = v2;
        let a544 = 60u64;
        nat_add(a543, a544)?
    })?))
}

pub fn f48(v0: Bytes) -> R<Adt8> {
    f12(f32(f21(f6(), f7()), v0)?, 32u64)
}

pub fn f49(v0: Adt8, v1: u64) -> Adt6 {
    f84(Fn12::F85(v1), Adt6::C0(0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32), match v0 {
        Adt8::C0(_, _, _, _, _, _, h545) => {
            h545
        }
    })
}

pub fn f50() -> u32 {
    64u32
}

pub fn f51(v0: u64) -> R<u64> {
    let a546 = v0;
    let a547 = 0u64;
    if nat_eq(a546, a547) {
        Ok(0u64)
    } else {
        let a553 = {
            let a550 = {
                let a548 = v0;
                let a549 = 1u64;
                nat_sub(a548, a549)
            };
            let a551 = 64u64;
            let a552 = 0u64;
            nat_quot(a550, a551, a552)
        };
        let a554 = 64u64;
        nat_mul(a553, a554)
    }
}

pub fn f52(v0: Fn7, p558: u64, p559: (u64, (Adt5, List<Adt3>))) -> R<(u64, (Adt5, List<Adt3>))> {
    let m557 = (p558, p559);
    {
        let mut m557 = m557;
        loop {
            let step = {
                let (v1, v2) = m557;
                let m555 = v1;
                if m555 == 0 {
                    Err::<(u64, (u64, (Adt5, List<Adt3>))), R<(u64, (Adt5, List<Adt3>))>>(Ok(v2))
                } else {
                    let v3 = m555 - 1;
                    Ok::<(u64, (u64, (Adt5, List<Adt3>))), R<(u64, (Adt5, List<Adt3>))>>((v3, {
                        let c556: Fn7 = v0.clone();
                        c556.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m557 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f53(v0: u32, v1: u64, v2: u64, v3: Adt5, v4: u64, v5: Bytes, v6: (u64, (Adt5, List<Adt3>))) -> R<(u64, (Adt5, List<Adt3>))> {
    let v7: u64 = {
        let (h560, _) = v6.clone();
        h560
    };
    let v8: Adt4 = f57(f54(v3, v0, v5, v4, v1, v2, v7, {
        let (h561, _) = {
            let (_, h562) = v6.clone();
            h562
        };
        h561
    }, 64u64, f56(v7))?)?;
    Ok(({
        let a563 = v7;
        let a564 = 64u64;
        nat_add(a563, a564)?
    }, (f46(match v8.clone() {
        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h565) => {
            h565
        }
    }), List::cons(Adt3::C0(v8), {
        let (_, h566) = {
            let (_, h567) = v6;
            h567
        };
        h566
    }))))
}

pub fn f54(_: Adt5, v1: u32, v2: Bytes, v3: u64, v4: u64, v5: u64, v6: u64, v7: Adt5, v8: u64, v9: u32) -> R<Adt10> {
    Ok(Adt10::C0(0u64, v5, 1u64, {
        let a568 = v6;
        let a569 = 64u64;
        let a570 = 0u64;
        nat_quot(a568, a569, a570)
    }, {
        let a573 = {
            let a571 = v5;
            let a572 = 1024u64;
            nat_mul(a571, a572)?
        };
        let a574 = v6;
        nat_add(a573, a574)?
    }, v7, f47(v2, v3, {
        let a575 = v4;
        let a576 = v6;
        nat_add(a575, a576)?
    })?, v5, f71(v8, 7u64), {
        let a577 = v1;
        let a578 = v9;
        fixed_u32::bit_or(a577, a578)
    }))
}

pub fn f55() -> u32 {
    2u32
}

pub fn f56(v0: u64) -> u32 {
    let a579 = v0;
    let a580 = 0u64;
    if nat_eq(a579, a580) {
        f86()
    } else {
        0u32
    }
}

pub fn f57(v0: Adt10) -> R<Adt4> {
    let v1: u32 = f65(match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, _, h581, _, _) => {
            h581
        }
    });
    let v2: u32 = f66(match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, _, h582, _, _) => {
            h582
        }
    });
    Ok(Adt4::C0(match v0.clone() {
        Adt10::C0(h583, _, _, _, _, _, _, _, _, _) => {
            h583
        }
    }, match v0.clone() {
        Adt10::C0(_, h584, _, _, _, _, _, _, _, _) => {
            h584
        }
    }, match v0.clone() {
        Adt10::C0(_, _, h585, _, _, _, _, _, _, _) => {
            h585
        }
    }, match v0.clone() {
        Adt10::C0(_, _, _, h586, _, _, _, _, _, _) => {
            h586
        }
    }, match v0.clone() {
        Adt10::C0(_, _, _, _, h587, _, _, _, _, _) => {
            h587
        }
    }, match v0.clone() {
        Adt10::C0(_, _, _, _, _, h588, _, _, _, _) => {
            h588
        }
    }, match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, h589, _, _, _) => {
            h589
        }
    }, v1, v2, match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, _, _, h590, _) => {
            h590
        }
    }, match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, _, _, _, h591) => {
            h591
        }
    }, f87(match v0.clone() {
        Adt10::C0(_, _, _, _, _, h592, _, _, _, _) => {
            h592
        }
    }, match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, h593, _, _, _) => {
            h593
        }
    }, v1, v2, match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, _, _, h594, _) => {
            h594
        }
    }, match v0 {
        Adt10::C0(_, _, _, _, _, _, _, _, _, h595) => {
            h595
        }
    })?))
}

pub fn f58(v0: Adt7, v1: Adt7, v2: Adt5, v3: u32) -> R<Adt10> {
    Ok(Adt10::C0(1u64, match v0.clone() {
        Adt7::C0(_, h596, _) => {
            h596
        }
    }, {
        let a599 = match v0.clone() {
            Adt7::C0(_, _, h597) => {
                h597
            }
        };
        let a600 = match v1.clone() {
            Adt7::C0(_, _, h598) => {
                h598
            }
        };
        nat_add(a599, a600)?
    }, 0u64, 0u64, v2, Adt6::C0({
        let m603 = match v0.clone() {
            Adt7::C0(h602, _, _) => {
                h602
            }
        };
        match m603 {
            Adt5::C0(h601, _, _, _, _, _, _, _) => {
                h601
            }
        }
    }, {
        let m606 = match v0.clone() {
            Adt7::C0(h605, _, _) => {
                h605
            }
        };
        match m606 {
            Adt5::C0(_, h604, _, _, _, _, _, _) => {
                h604
            }
        }
    }, {
        let m609 = match v0.clone() {
            Adt7::C0(h608, _, _) => {
                h608
            }
        };
        match m609 {
            Adt5::C0(_, _, h607, _, _, _, _, _) => {
                h607
            }
        }
    }, {
        let m612 = match v0.clone() {
            Adt7::C0(h611, _, _) => {
                h611
            }
        };
        match m612 {
            Adt5::C0(_, _, _, h610, _, _, _, _) => {
                h610
            }
        }
    }, {
        let m615 = match v0.clone() {
            Adt7::C0(h614, _, _) => {
                h614
            }
        };
        match m615 {
            Adt5::C0(_, _, _, _, h613, _, _, _) => {
                h613
            }
        }
    }, {
        let m618 = match v0.clone() {
            Adt7::C0(h617, _, _) => {
                h617
            }
        };
        match m618 {
            Adt5::C0(_, _, _, _, _, h616, _, _) => {
                h616
            }
        }
    }, {
        let m621 = match v0.clone() {
            Adt7::C0(h620, _, _) => {
                h620
            }
        };
        match m621 {
            Adt5::C0(_, _, _, _, _, _, h619, _) => {
                h619
            }
        }
    }, {
        let m624 = match v0 {
            Adt7::C0(h623, _, _) => {
                h623
            }
        };
        match m624 {
            Adt5::C0(_, _, _, _, _, _, _, h622) => {
                h622
            }
        }
    }, {
        let m627 = match v1.clone() {
            Adt7::C0(h626, _, _) => {
                h626
            }
        };
        match m627 {
            Adt5::C0(h625, _, _, _, _, _, _, _) => {
                h625
            }
        }
    }, {
        let m630 = match v1.clone() {
            Adt7::C0(h629, _, _) => {
                h629
            }
        };
        match m630 {
            Adt5::C0(_, h628, _, _, _, _, _, _) => {
                h628
            }
        }
    }, {
        let m633 = match v1.clone() {
            Adt7::C0(h632, _, _) => {
                h632
            }
        };
        match m633 {
            Adt5::C0(_, _, h631, _, _, _, _, _) => {
                h631
            }
        }
    }, {
        let m636 = match v1.clone() {
            Adt7::C0(h635, _, _) => {
                h635
            }
        };
        match m636 {
            Adt5::C0(_, _, _, h634, _, _, _, _) => {
                h634
            }
        }
    }, {
        let m639 = match v1.clone() {
            Adt7::C0(h638, _, _) => {
                h638
            }
        };
        match m639 {
            Adt5::C0(_, _, _, _, h637, _, _, _) => {
                h637
            }
        }
    }, {
        let m642 = match v1.clone() {
            Adt7::C0(h641, _, _) => {
                h641
            }
        };
        match m642 {
            Adt5::C0(_, _, _, _, _, h640, _, _) => {
                h640
            }
        }
    }, {
        let m645 = match v1.clone() {
            Adt7::C0(h644, _, _) => {
                h644
            }
        };
        match m645 {
            Adt5::C0(_, _, _, _, _, _, h643, _) => {
                h643
            }
        }
    }, {
        let m648 = match v1 {
            Adt7::C0(h647, _, _) => {
                h647
            }
        };
        match m648 {
            Adt5::C0(_, _, _, _, _, _, _, h646) => {
                h646
            }
        }
    }), 0u64, 64u32, {
        let a649 = v3;
        let a650 = f88();
        fixed_u32::bit_or(a649, a650)
    }))
}

pub fn f59(v0: Adt10, v1: u64) -> Adt10 {
    Adt10::C0(2u64, match v0.clone() {
        Adt10::C0(_, h651, _, _, _, _, _, _, _, _) => {
            h651
        }
    }, match v0.clone() {
        Adt10::C0(_, _, h652, _, _, _, _, _, _, _) => {
            h652
        }
    }, v1, match v0.clone() {
        Adt10::C0(_, _, _, _, h653, _, _, _, _, _) => {
            h653
        }
    }, match v0.clone() {
        Adt10::C0(_, _, _, _, _, h654, _, _, _, _) => {
            h654
        }
    }, match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, h655, _, _, _) => {
            h655
        }
    }, v1, match v0.clone() {
        Adt10::C0(_, _, _, _, _, _, _, _, h656, _) => {
            h656
        }
    }, {
        let a658 = match v0 {
            Adt10::C0(_, _, _, _, _, _, _, _, _, h657) => {
                h657
            }
        };
        let a659 = f89();
        fixed_u32::bit_or(a658, a659)
    })
}

pub fn f60(v0: Fn8, p663: List<Adt3>, p664: List<Adt3>) -> List<Adt3> {
    let m662 = (p663, p664);
    {
        let mut m662 = m662;
        loop {
            let step = {
                let (v1, v2) = m662;
                let m660 = v2;
                match m660.uncons() {
                    None => {
                        Err::<(List<Adt3>, List<Adt3>), List<Adt3>>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(List<Adt3>, List<Adt3>), List<Adt3>>(({
                            let c661: Fn8 = v0.clone();
                            c661.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m662 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f61(v0: List<Adt3>, v1: Adt3) -> List<Adt3> {
    List::cons(v1, v0)
}

pub fn f62(v0: Fn9, p668: u64, p669: (u64, (List<Adt7>, List<Adt3>))) -> R<(u64, (List<Adt7>, List<Adt3>))> {
    let m667 = (p668, p669);
    {
        let mut m667 = m667;
        loop {
            let step = {
                let (v1, v2) = m667;
                let m665 = v1;
                if m665 == 0 {
                    Err::<(u64, (u64, (List<Adt7>, List<Adt3>))), R<(u64, (List<Adt7>, List<Adt3>))>>(Ok(v2))
                } else {
                    let v3 = m665 - 1;
                    Ok::<(u64, (u64, (List<Adt7>, List<Adt3>))), R<(u64, (List<Adt7>, List<Adt3>))>>((v3, {
                        let c666: Fn9 = v0.clone();
                        c666.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m667 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f63(v0: u32, v1: Bytes, v2: Adt5, v3: Adt9, v4: (u64, (List<Adt7>, List<Adt3>))) -> R<(u64, (List<Adt7>, List<Adt3>))> {
    let v5: u64 = {
        let a672 = match v3 {
            Adt9::C0(_, _, h670, _, _, _, _) => {
                h670
            }
        };
        let a673 = {
            let (h671, _) = v4.clone();
            h671
        };
        nat_add(a672, a673)?
    };
    let v6: (Adt10, List<Adt3>) = f25(v2.clone(), v0, v1, {
        let a675 = {
            let (h674, _) = v4.clone();
            h674
        };
        let a676 = 1024u64;
        nat_mul(a675, a676)?
    }, 1024u64, v5, {
        let (_, h677) = {
            let (_, h678) = v4.clone();
            h678
        };
        h677
    })?;
    let v7: Adt4 = f57({
        let (h679, _) = v6;
        h679
    })?;
    Ok(({
        let a681 = {
            let (h680, _) = v4;
            h680
        };
        let a682 = 1u64;
        nat_add(a681, a682)?
    }, f90(v2, v0, Adt12::C0(Adt7::C0(f46(match v7.clone() {
        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h683) => {
            h683
        }
    }), v5, 1u64), {
        let a684 = v5;
        let a685 = 1u64;
        nat_add(a684, a685)?
    }, {
        let (h686, _) = {
            let (_, h687) = v4;
            h687
        };
        h686
    }, List::cons(Adt3::C0(v7), {
        let (_, h688) = v6;
        h688
    })))?))
}

pub fn f64(v0: u32) -> Bytes {
    let a699 = f70({
        let a689 = v0;
        let a690 = 255u32;
        fixed_u32::bit_and(a689, a690)
    });
    let a700 = {
        let a697 = f70({
            let a691 = f82(v0, 8u32);
            let a692 = 255u32;
            fixed_u32::bit_and(a691, a692)
        });
        let a698 = {
            let a695 = f70({
                let a693 = f82(v0, 16u32);
                let a694 = 255u32;
                fixed_u32::bit_and(a693, a694)
            });
            let a696 = f70(f82(v0, 24u32));
            append_bytes(a695, a696)
        };
        append_bytes(a697, a698)
    };
    append_bytes(a699, a700)
}

pub fn f65(v0: u64) -> u32 {
    f71({
        let a701 = v0;
        let a702 = 4294967296u64;
        let a703 = 0u64;
        nat_rem(a701, a702, a703)
    }, 32u64)
}

pub fn f66(v0: u64) -> u32 {
    let a704 = v0;
    let a705 = 4294967296u64;
    if nat_lt(a704, a705) {
        0u32
    } else {
        f71({
            let a706 = v0;
            let a707 = 4294967296u64;
            let a708 = 0u64;
            nat_quot(a706, a707, a708)
        }, 32u64)
    }
}

pub fn f67(v0: Bytes, v1: Bytes) -> Bytes {
    let a712 = f91({
        let a709 = v0.clone();
        length_bytes(a709)
    });
    let a713 = {
        let a710 = v0;
        let a711 = v1;
        append_bytes(a710, a711)
    };
    append_bytes(a712, a713)
}

pub fn f68(v0: Bytes) -> Bytes {
    let m717 = {
        let a714 = v0.clone();
        let a715 = 0u64;
        let a716 = 32u64;
        slice_bytes(a714, a715, a716)
    };
    match m717 {
        None => {
            v0
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f69(v0: List<Bytes>, v1: Bytes) -> List<Bytes> {
    List::cons(v1, v0)
}

pub fn f70(v0: u32) -> Bytes {
    let a720 = {
        let a718 = v0;
        let a719 = 128u32;
        fixed_u32::bit_and(a718, a719)
    };
    let a721 = 0u32;
    if equal(a720, a721) {
        let a724 = {
            let a722 = v0;
            let a723 = 64u32;
            fixed_u32::bit_and(a722, a723)
        };
        let a725 = 0u32;
        if equal(a724, a725) {
            let a728 = {
                let a726 = v0;
                let a727 = 32u32;
                fixed_u32::bit_and(a726, a727)
            };
            let a729 = 0u32;
            if equal(a728, a729) {
                let a732 = {
                    let a730 = v0;
                    let a731 = 16u32;
                    fixed_u32::bit_and(a730, a731)
                };
                let a733 = 0u32;
                if equal(a732, a733) {
                    let a736 = {
                        let a734 = v0;
                        let a735 = 8u32;
                        fixed_u32::bit_and(a734, a735)
                    };
                    let a737 = 0u32;
                    if equal(a736, a737) {
                        let a740 = {
                            let a738 = v0;
                            let a739 = 4u32;
                            fixed_u32::bit_and(a738, a739)
                        };
                        let a741 = 0u32;
                        if equal(a740, a741) {
                            let a744 = {
                                let a742 = v0;
                                let a743 = 2u32;
                                fixed_u32::bit_and(a742, a743)
                            };
                            let a745 = 0u32;
                            if equal(a744, a745) {
                                let a748 = {
                                    let a746 = v0;
                                    let a747 = 1u32;
                                    fixed_u32::bit_and(a746, a747)
                                };
                                let a749 = 0u32;
                                if equal(a748, a749) {
                                    Bytes::lit(&[0x00])
                                } else {
                                    Bytes::lit(&[0x01])
                                }
                            } else {
                                let a752 = {
                                    let a750 = v0;
                                    let a751 = 1u32;
                                    fixed_u32::bit_and(a750, a751)
                                };
                                let a753 = 0u32;
                                if equal(a752, a753) {
                                    Bytes::lit(&[0x02])
                                } else {
                                    Bytes::lit(&[0x03])
                                }
                            }
                        } else {
                            let a756 = {
                                let a754 = v0;
                                let a755 = 2u32;
                                fixed_u32::bit_and(a754, a755)
                            };
                            let a757 = 0u32;
                            if equal(a756, a757) {
                                let a760 = {
                                    let a758 = v0;
                                    let a759 = 1u32;
                                    fixed_u32::bit_and(a758, a759)
                                };
                                let a761 = 0u32;
                                if equal(a760, a761) {
                                    Bytes::lit(&[0x04])
                                } else {
                                    Bytes::lit(&[0x05])
                                }
                            } else {
                                let a764 = {
                                    let a762 = v0;
                                    let a763 = 1u32;
                                    fixed_u32::bit_and(a762, a763)
                                };
                                let a765 = 0u32;
                                if equal(a764, a765) {
                                    Bytes::lit(&[0x06])
                                } else {
                                    Bytes::lit(&[0x07])
                                }
                            }
                        }
                    } else {
                        let a768 = {
                            let a766 = v0;
                            let a767 = 4u32;
                            fixed_u32::bit_and(a766, a767)
                        };
                        let a769 = 0u32;
                        if equal(a768, a769) {
                            let a772 = {
                                let a770 = v0;
                                let a771 = 2u32;
                                fixed_u32::bit_and(a770, a771)
                            };
                            let a773 = 0u32;
                            if equal(a772, a773) {
                                let a776 = {
                                    let a774 = v0;
                                    let a775 = 1u32;
                                    fixed_u32::bit_and(a774, a775)
                                };
                                let a777 = 0u32;
                                if equal(a776, a777) {
                                    Bytes::lit(&[0x08])
                                } else {
                                    Bytes::lit(&[0x09])
                                }
                            } else {
                                let a780 = {
                                    let a778 = v0;
                                    let a779 = 1u32;
                                    fixed_u32::bit_and(a778, a779)
                                };
                                let a781 = 0u32;
                                if equal(a780, a781) {
                                    Bytes::lit(&[0x0a])
                                } else {
                                    Bytes::lit(&[0x0b])
                                }
                            }
                        } else {
                            let a784 = {
                                let a782 = v0;
                                let a783 = 2u32;
                                fixed_u32::bit_and(a782, a783)
                            };
                            let a785 = 0u32;
                            if equal(a784, a785) {
                                let a788 = {
                                    let a786 = v0;
                                    let a787 = 1u32;
                                    fixed_u32::bit_and(a786, a787)
                                };
                                let a789 = 0u32;
                                if equal(a788, a789) {
                                    Bytes::lit(&[0x0c])
                                } else {
                                    Bytes::lit(&[0x0d])
                                }
                            } else {
                                let a792 = {
                                    let a790 = v0;
                                    let a791 = 1u32;
                                    fixed_u32::bit_and(a790, a791)
                                };
                                let a793 = 0u32;
                                if equal(a792, a793) {
                                    Bytes::lit(&[0x0e])
                                } else {
                                    Bytes::lit(&[0x0f])
                                }
                            }
                        }
                    }
                } else {
                    let a796 = {
                        let a794 = v0;
                        let a795 = 8u32;
                        fixed_u32::bit_and(a794, a795)
                    };
                    let a797 = 0u32;
                    if equal(a796, a797) {
                        let a800 = {
                            let a798 = v0;
                            let a799 = 4u32;
                            fixed_u32::bit_and(a798, a799)
                        };
                        let a801 = 0u32;
                        if equal(a800, a801) {
                            let a804 = {
                                let a802 = v0;
                                let a803 = 2u32;
                                fixed_u32::bit_and(a802, a803)
                            };
                            let a805 = 0u32;
                            if equal(a804, a805) {
                                let a808 = {
                                    let a806 = v0;
                                    let a807 = 1u32;
                                    fixed_u32::bit_and(a806, a807)
                                };
                                let a809 = 0u32;
                                if equal(a808, a809) {
                                    Bytes::lit(&[0x10])
                                } else {
                                    Bytes::lit(&[0x11])
                                }
                            } else {
                                let a812 = {
                                    let a810 = v0;
                                    let a811 = 1u32;
                                    fixed_u32::bit_and(a810, a811)
                                };
                                let a813 = 0u32;
                                if equal(a812, a813) {
                                    Bytes::lit(&[0x12])
                                } else {
                                    Bytes::lit(&[0x13])
                                }
                            }
                        } else {
                            let a816 = {
                                let a814 = v0;
                                let a815 = 2u32;
                                fixed_u32::bit_and(a814, a815)
                            };
                            let a817 = 0u32;
                            if equal(a816, a817) {
                                let a820 = {
                                    let a818 = v0;
                                    let a819 = 1u32;
                                    fixed_u32::bit_and(a818, a819)
                                };
                                let a821 = 0u32;
                                if equal(a820, a821) {
                                    Bytes::lit(&[0x14])
                                } else {
                                    Bytes::lit(&[0x15])
                                }
                            } else {
                                let a824 = {
                                    let a822 = v0;
                                    let a823 = 1u32;
                                    fixed_u32::bit_and(a822, a823)
                                };
                                let a825 = 0u32;
                                if equal(a824, a825) {
                                    Bytes::lit(&[0x16])
                                } else {
                                    Bytes::lit(&[0x17])
                                }
                            }
                        }
                    } else {
                        let a828 = {
                            let a826 = v0;
                            let a827 = 4u32;
                            fixed_u32::bit_and(a826, a827)
                        };
                        let a829 = 0u32;
                        if equal(a828, a829) {
                            let a832 = {
                                let a830 = v0;
                                let a831 = 2u32;
                                fixed_u32::bit_and(a830, a831)
                            };
                            let a833 = 0u32;
                            if equal(a832, a833) {
                                let a836 = {
                                    let a834 = v0;
                                    let a835 = 1u32;
                                    fixed_u32::bit_and(a834, a835)
                                };
                                let a837 = 0u32;
                                if equal(a836, a837) {
                                    Bytes::lit(&[0x18])
                                } else {
                                    Bytes::lit(&[0x19])
                                }
                            } else {
                                let a840 = {
                                    let a838 = v0;
                                    let a839 = 1u32;
                                    fixed_u32::bit_and(a838, a839)
                                };
                                let a841 = 0u32;
                                if equal(a840, a841) {
                                    Bytes::lit(&[0x1a])
                                } else {
                                    Bytes::lit(&[0x1b])
                                }
                            }
                        } else {
                            let a844 = {
                                let a842 = v0;
                                let a843 = 2u32;
                                fixed_u32::bit_and(a842, a843)
                            };
                            let a845 = 0u32;
                            if equal(a844, a845) {
                                let a848 = {
                                    let a846 = v0;
                                    let a847 = 1u32;
                                    fixed_u32::bit_and(a846, a847)
                                };
                                let a849 = 0u32;
                                if equal(a848, a849) {
                                    Bytes::lit(&[0x1c])
                                } else {
                                    Bytes::lit(&[0x1d])
                                }
                            } else {
                                let a852 = {
                                    let a850 = v0;
                                    let a851 = 1u32;
                                    fixed_u32::bit_and(a850, a851)
                                };
                                let a853 = 0u32;
                                if equal(a852, a853) {
                                    Bytes::lit(&[0x1e])
                                } else {
                                    Bytes::lit(&[0x1f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a856 = {
                    let a854 = v0;
                    let a855 = 16u32;
                    fixed_u32::bit_and(a854, a855)
                };
                let a857 = 0u32;
                if equal(a856, a857) {
                    let a860 = {
                        let a858 = v0;
                        let a859 = 8u32;
                        fixed_u32::bit_and(a858, a859)
                    };
                    let a861 = 0u32;
                    if equal(a860, a861) {
                        let a864 = {
                            let a862 = v0;
                            let a863 = 4u32;
                            fixed_u32::bit_and(a862, a863)
                        };
                        let a865 = 0u32;
                        if equal(a864, a865) {
                            let a868 = {
                                let a866 = v0;
                                let a867 = 2u32;
                                fixed_u32::bit_and(a866, a867)
                            };
                            let a869 = 0u32;
                            if equal(a868, a869) {
                                let a872 = {
                                    let a870 = v0;
                                    let a871 = 1u32;
                                    fixed_u32::bit_and(a870, a871)
                                };
                                let a873 = 0u32;
                                if equal(a872, a873) {
                                    Bytes::lit(&[0x20])
                                } else {
                                    Bytes::lit(&[0x21])
                                }
                            } else {
                                let a876 = {
                                    let a874 = v0;
                                    let a875 = 1u32;
                                    fixed_u32::bit_and(a874, a875)
                                };
                                let a877 = 0u32;
                                if equal(a876, a877) {
                                    Bytes::lit(&[0x22])
                                } else {
                                    Bytes::lit(&[0x23])
                                }
                            }
                        } else {
                            let a880 = {
                                let a878 = v0;
                                let a879 = 2u32;
                                fixed_u32::bit_and(a878, a879)
                            };
                            let a881 = 0u32;
                            if equal(a880, a881) {
                                let a884 = {
                                    let a882 = v0;
                                    let a883 = 1u32;
                                    fixed_u32::bit_and(a882, a883)
                                };
                                let a885 = 0u32;
                                if equal(a884, a885) {
                                    Bytes::lit(&[0x24])
                                } else {
                                    Bytes::lit(&[0x25])
                                }
                            } else {
                                let a888 = {
                                    let a886 = v0;
                                    let a887 = 1u32;
                                    fixed_u32::bit_and(a886, a887)
                                };
                                let a889 = 0u32;
                                if equal(a888, a889) {
                                    Bytes::lit(&[0x26])
                                } else {
                                    Bytes::lit(&[0x27])
                                }
                            }
                        }
                    } else {
                        let a892 = {
                            let a890 = v0;
                            let a891 = 4u32;
                            fixed_u32::bit_and(a890, a891)
                        };
                        let a893 = 0u32;
                        if equal(a892, a893) {
                            let a896 = {
                                let a894 = v0;
                                let a895 = 2u32;
                                fixed_u32::bit_and(a894, a895)
                            };
                            let a897 = 0u32;
                            if equal(a896, a897) {
                                let a900 = {
                                    let a898 = v0;
                                    let a899 = 1u32;
                                    fixed_u32::bit_and(a898, a899)
                                };
                                let a901 = 0u32;
                                if equal(a900, a901) {
                                    Bytes::lit(&[0x28])
                                } else {
                                    Bytes::lit(&[0x29])
                                }
                            } else {
                                let a904 = {
                                    let a902 = v0;
                                    let a903 = 1u32;
                                    fixed_u32::bit_and(a902, a903)
                                };
                                let a905 = 0u32;
                                if equal(a904, a905) {
                                    Bytes::lit(&[0x2a])
                                } else {
                                    Bytes::lit(&[0x2b])
                                }
                            }
                        } else {
                            let a908 = {
                                let a906 = v0;
                                let a907 = 2u32;
                                fixed_u32::bit_and(a906, a907)
                            };
                            let a909 = 0u32;
                            if equal(a908, a909) {
                                let a912 = {
                                    let a910 = v0;
                                    let a911 = 1u32;
                                    fixed_u32::bit_and(a910, a911)
                                };
                                let a913 = 0u32;
                                if equal(a912, a913) {
                                    Bytes::lit(&[0x2c])
                                } else {
                                    Bytes::lit(&[0x2d])
                                }
                            } else {
                                let a916 = {
                                    let a914 = v0;
                                    let a915 = 1u32;
                                    fixed_u32::bit_and(a914, a915)
                                };
                                let a917 = 0u32;
                                if equal(a916, a917) {
                                    Bytes::lit(&[0x2e])
                                } else {
                                    Bytes::lit(&[0x2f])
                                }
                            }
                        }
                    }
                } else {
                    let a920 = {
                        let a918 = v0;
                        let a919 = 8u32;
                        fixed_u32::bit_and(a918, a919)
                    };
                    let a921 = 0u32;
                    if equal(a920, a921) {
                        let a924 = {
                            let a922 = v0;
                            let a923 = 4u32;
                            fixed_u32::bit_and(a922, a923)
                        };
                        let a925 = 0u32;
                        if equal(a924, a925) {
                            let a928 = {
                                let a926 = v0;
                                let a927 = 2u32;
                                fixed_u32::bit_and(a926, a927)
                            };
                            let a929 = 0u32;
                            if equal(a928, a929) {
                                let a932 = {
                                    let a930 = v0;
                                    let a931 = 1u32;
                                    fixed_u32::bit_and(a930, a931)
                                };
                                let a933 = 0u32;
                                if equal(a932, a933) {
                                    Bytes::lit(&[0x30])
                                } else {
                                    Bytes::lit(&[0x31])
                                }
                            } else {
                                let a936 = {
                                    let a934 = v0;
                                    let a935 = 1u32;
                                    fixed_u32::bit_and(a934, a935)
                                };
                                let a937 = 0u32;
                                if equal(a936, a937) {
                                    Bytes::lit(&[0x32])
                                } else {
                                    Bytes::lit(&[0x33])
                                }
                            }
                        } else {
                            let a940 = {
                                let a938 = v0;
                                let a939 = 2u32;
                                fixed_u32::bit_and(a938, a939)
                            };
                            let a941 = 0u32;
                            if equal(a940, a941) {
                                let a944 = {
                                    let a942 = v0;
                                    let a943 = 1u32;
                                    fixed_u32::bit_and(a942, a943)
                                };
                                let a945 = 0u32;
                                if equal(a944, a945) {
                                    Bytes::lit(&[0x34])
                                } else {
                                    Bytes::lit(&[0x35])
                                }
                            } else {
                                let a948 = {
                                    let a946 = v0;
                                    let a947 = 1u32;
                                    fixed_u32::bit_and(a946, a947)
                                };
                                let a949 = 0u32;
                                if equal(a948, a949) {
                                    Bytes::lit(&[0x36])
                                } else {
                                    Bytes::lit(&[0x37])
                                }
                            }
                        }
                    } else {
                        let a952 = {
                            let a950 = v0;
                            let a951 = 4u32;
                            fixed_u32::bit_and(a950, a951)
                        };
                        let a953 = 0u32;
                        if equal(a952, a953) {
                            let a956 = {
                                let a954 = v0;
                                let a955 = 2u32;
                                fixed_u32::bit_and(a954, a955)
                            };
                            let a957 = 0u32;
                            if equal(a956, a957) {
                                let a960 = {
                                    let a958 = v0;
                                    let a959 = 1u32;
                                    fixed_u32::bit_and(a958, a959)
                                };
                                let a961 = 0u32;
                                if equal(a960, a961) {
                                    Bytes::lit(&[0x38])
                                } else {
                                    Bytes::lit(&[0x39])
                                }
                            } else {
                                let a964 = {
                                    let a962 = v0;
                                    let a963 = 1u32;
                                    fixed_u32::bit_and(a962, a963)
                                };
                                let a965 = 0u32;
                                if equal(a964, a965) {
                                    Bytes::lit(&[0x3a])
                                } else {
                                    Bytes::lit(&[0x3b])
                                }
                            }
                        } else {
                            let a968 = {
                                let a966 = v0;
                                let a967 = 2u32;
                                fixed_u32::bit_and(a966, a967)
                            };
                            let a969 = 0u32;
                            if equal(a968, a969) {
                                let a972 = {
                                    let a970 = v0;
                                    let a971 = 1u32;
                                    fixed_u32::bit_and(a970, a971)
                                };
                                let a973 = 0u32;
                                if equal(a972, a973) {
                                    Bytes::lit(&[0x3c])
                                } else {
                                    Bytes::lit(&[0x3d])
                                }
                            } else {
                                let a976 = {
                                    let a974 = v0;
                                    let a975 = 1u32;
                                    fixed_u32::bit_and(a974, a975)
                                };
                                let a977 = 0u32;
                                if equal(a976, a977) {
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
            let a980 = {
                let a978 = v0;
                let a979 = 32u32;
                fixed_u32::bit_and(a978, a979)
            };
            let a981 = 0u32;
            if equal(a980, a981) {
                let a984 = {
                    let a982 = v0;
                    let a983 = 16u32;
                    fixed_u32::bit_and(a982, a983)
                };
                let a985 = 0u32;
                if equal(a984, a985) {
                    let a988 = {
                        let a986 = v0;
                        let a987 = 8u32;
                        fixed_u32::bit_and(a986, a987)
                    };
                    let a989 = 0u32;
                    if equal(a988, a989) {
                        let a992 = {
                            let a990 = v0;
                            let a991 = 4u32;
                            fixed_u32::bit_and(a990, a991)
                        };
                        let a993 = 0u32;
                        if equal(a992, a993) {
                            let a996 = {
                                let a994 = v0;
                                let a995 = 2u32;
                                fixed_u32::bit_and(a994, a995)
                            };
                            let a997 = 0u32;
                            if equal(a996, a997) {
                                let a1000 = {
                                    let a998 = v0;
                                    let a999 = 1u32;
                                    fixed_u32::bit_and(a998, a999)
                                };
                                let a1001 = 0u32;
                                if equal(a1000, a1001) {
                                    Bytes::lit(&[0x40])
                                } else {
                                    Bytes::lit(&[0x41])
                                }
                            } else {
                                let a1004 = {
                                    let a1002 = v0;
                                    let a1003 = 1u32;
                                    fixed_u32::bit_and(a1002, a1003)
                                };
                                let a1005 = 0u32;
                                if equal(a1004, a1005) {
                                    Bytes::lit(&[0x42])
                                } else {
                                    Bytes::lit(&[0x43])
                                }
                            }
                        } else {
                            let a1008 = {
                                let a1006 = v0;
                                let a1007 = 2u32;
                                fixed_u32::bit_and(a1006, a1007)
                            };
                            let a1009 = 0u32;
                            if equal(a1008, a1009) {
                                let a1012 = {
                                    let a1010 = v0;
                                    let a1011 = 1u32;
                                    fixed_u32::bit_and(a1010, a1011)
                                };
                                let a1013 = 0u32;
                                if equal(a1012, a1013) {
                                    Bytes::lit(&[0x44])
                                } else {
                                    Bytes::lit(&[0x45])
                                }
                            } else {
                                let a1016 = {
                                    let a1014 = v0;
                                    let a1015 = 1u32;
                                    fixed_u32::bit_and(a1014, a1015)
                                };
                                let a1017 = 0u32;
                                if equal(a1016, a1017) {
                                    Bytes::lit(&[0x46])
                                } else {
                                    Bytes::lit(&[0x47])
                                }
                            }
                        }
                    } else {
                        let a1020 = {
                            let a1018 = v0;
                            let a1019 = 4u32;
                            fixed_u32::bit_and(a1018, a1019)
                        };
                        let a1021 = 0u32;
                        if equal(a1020, a1021) {
                            let a1024 = {
                                let a1022 = v0;
                                let a1023 = 2u32;
                                fixed_u32::bit_and(a1022, a1023)
                            };
                            let a1025 = 0u32;
                            if equal(a1024, a1025) {
                                let a1028 = {
                                    let a1026 = v0;
                                    let a1027 = 1u32;
                                    fixed_u32::bit_and(a1026, a1027)
                                };
                                let a1029 = 0u32;
                                if equal(a1028, a1029) {
                                    Bytes::lit(&[0x48])
                                } else {
                                    Bytes::lit(&[0x49])
                                }
                            } else {
                                let a1032 = {
                                    let a1030 = v0;
                                    let a1031 = 1u32;
                                    fixed_u32::bit_and(a1030, a1031)
                                };
                                let a1033 = 0u32;
                                if equal(a1032, a1033) {
                                    Bytes::lit(&[0x4a])
                                } else {
                                    Bytes::lit(&[0x4b])
                                }
                            }
                        } else {
                            let a1036 = {
                                let a1034 = v0;
                                let a1035 = 2u32;
                                fixed_u32::bit_and(a1034, a1035)
                            };
                            let a1037 = 0u32;
                            if equal(a1036, a1037) {
                                let a1040 = {
                                    let a1038 = v0;
                                    let a1039 = 1u32;
                                    fixed_u32::bit_and(a1038, a1039)
                                };
                                let a1041 = 0u32;
                                if equal(a1040, a1041) {
                                    Bytes::lit(&[0x4c])
                                } else {
                                    Bytes::lit(&[0x4d])
                                }
                            } else {
                                let a1044 = {
                                    let a1042 = v0;
                                    let a1043 = 1u32;
                                    fixed_u32::bit_and(a1042, a1043)
                                };
                                let a1045 = 0u32;
                                if equal(a1044, a1045) {
                                    Bytes::lit(&[0x4e])
                                } else {
                                    Bytes::lit(&[0x4f])
                                }
                            }
                        }
                    }
                } else {
                    let a1048 = {
                        let a1046 = v0;
                        let a1047 = 8u32;
                        fixed_u32::bit_and(a1046, a1047)
                    };
                    let a1049 = 0u32;
                    if equal(a1048, a1049) {
                        let a1052 = {
                            let a1050 = v0;
                            let a1051 = 4u32;
                            fixed_u32::bit_and(a1050, a1051)
                        };
                        let a1053 = 0u32;
                        if equal(a1052, a1053) {
                            let a1056 = {
                                let a1054 = v0;
                                let a1055 = 2u32;
                                fixed_u32::bit_and(a1054, a1055)
                            };
                            let a1057 = 0u32;
                            if equal(a1056, a1057) {
                                let a1060 = {
                                    let a1058 = v0;
                                    let a1059 = 1u32;
                                    fixed_u32::bit_and(a1058, a1059)
                                };
                                let a1061 = 0u32;
                                if equal(a1060, a1061) {
                                    Bytes::lit(&[0x50])
                                } else {
                                    Bytes::lit(&[0x51])
                                }
                            } else {
                                let a1064 = {
                                    let a1062 = v0;
                                    let a1063 = 1u32;
                                    fixed_u32::bit_and(a1062, a1063)
                                };
                                let a1065 = 0u32;
                                if equal(a1064, a1065) {
                                    Bytes::lit(&[0x52])
                                } else {
                                    Bytes::lit(&[0x53])
                                }
                            }
                        } else {
                            let a1068 = {
                                let a1066 = v0;
                                let a1067 = 2u32;
                                fixed_u32::bit_and(a1066, a1067)
                            };
                            let a1069 = 0u32;
                            if equal(a1068, a1069) {
                                let a1072 = {
                                    let a1070 = v0;
                                    let a1071 = 1u32;
                                    fixed_u32::bit_and(a1070, a1071)
                                };
                                let a1073 = 0u32;
                                if equal(a1072, a1073) {
                                    Bytes::lit(&[0x54])
                                } else {
                                    Bytes::lit(&[0x55])
                                }
                            } else {
                                let a1076 = {
                                    let a1074 = v0;
                                    let a1075 = 1u32;
                                    fixed_u32::bit_and(a1074, a1075)
                                };
                                let a1077 = 0u32;
                                if equal(a1076, a1077) {
                                    Bytes::lit(&[0x56])
                                } else {
                                    Bytes::lit(&[0x57])
                                }
                            }
                        }
                    } else {
                        let a1080 = {
                            let a1078 = v0;
                            let a1079 = 4u32;
                            fixed_u32::bit_and(a1078, a1079)
                        };
                        let a1081 = 0u32;
                        if equal(a1080, a1081) {
                            let a1084 = {
                                let a1082 = v0;
                                let a1083 = 2u32;
                                fixed_u32::bit_and(a1082, a1083)
                            };
                            let a1085 = 0u32;
                            if equal(a1084, a1085) {
                                let a1088 = {
                                    let a1086 = v0;
                                    let a1087 = 1u32;
                                    fixed_u32::bit_and(a1086, a1087)
                                };
                                let a1089 = 0u32;
                                if equal(a1088, a1089) {
                                    Bytes::lit(&[0x58])
                                } else {
                                    Bytes::lit(&[0x59])
                                }
                            } else {
                                let a1092 = {
                                    let a1090 = v0;
                                    let a1091 = 1u32;
                                    fixed_u32::bit_and(a1090, a1091)
                                };
                                let a1093 = 0u32;
                                if equal(a1092, a1093) {
                                    Bytes::lit(&[0x5a])
                                } else {
                                    Bytes::lit(&[0x5b])
                                }
                            }
                        } else {
                            let a1096 = {
                                let a1094 = v0;
                                let a1095 = 2u32;
                                fixed_u32::bit_and(a1094, a1095)
                            };
                            let a1097 = 0u32;
                            if equal(a1096, a1097) {
                                let a1100 = {
                                    let a1098 = v0;
                                    let a1099 = 1u32;
                                    fixed_u32::bit_and(a1098, a1099)
                                };
                                let a1101 = 0u32;
                                if equal(a1100, a1101) {
                                    Bytes::lit(&[0x5c])
                                } else {
                                    Bytes::lit(&[0x5d])
                                }
                            } else {
                                let a1104 = {
                                    let a1102 = v0;
                                    let a1103 = 1u32;
                                    fixed_u32::bit_and(a1102, a1103)
                                };
                                let a1105 = 0u32;
                                if equal(a1104, a1105) {
                                    Bytes::lit(&[0x5e])
                                } else {
                                    Bytes::lit(&[0x5f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a1108 = {
                    let a1106 = v0;
                    let a1107 = 16u32;
                    fixed_u32::bit_and(a1106, a1107)
                };
                let a1109 = 0u32;
                if equal(a1108, a1109) {
                    let a1112 = {
                        let a1110 = v0;
                        let a1111 = 8u32;
                        fixed_u32::bit_and(a1110, a1111)
                    };
                    let a1113 = 0u32;
                    if equal(a1112, a1113) {
                        let a1116 = {
                            let a1114 = v0;
                            let a1115 = 4u32;
                            fixed_u32::bit_and(a1114, a1115)
                        };
                        let a1117 = 0u32;
                        if equal(a1116, a1117) {
                            let a1120 = {
                                let a1118 = v0;
                                let a1119 = 2u32;
                                fixed_u32::bit_and(a1118, a1119)
                            };
                            let a1121 = 0u32;
                            if equal(a1120, a1121) {
                                let a1124 = {
                                    let a1122 = v0;
                                    let a1123 = 1u32;
                                    fixed_u32::bit_and(a1122, a1123)
                                };
                                let a1125 = 0u32;
                                if equal(a1124, a1125) {
                                    Bytes::lit(&[0x60])
                                } else {
                                    Bytes::lit(&[0x61])
                                }
                            } else {
                                let a1128 = {
                                    let a1126 = v0;
                                    let a1127 = 1u32;
                                    fixed_u32::bit_and(a1126, a1127)
                                };
                                let a1129 = 0u32;
                                if equal(a1128, a1129) {
                                    Bytes::lit(&[0x62])
                                } else {
                                    Bytes::lit(&[0x63])
                                }
                            }
                        } else {
                            let a1132 = {
                                let a1130 = v0;
                                let a1131 = 2u32;
                                fixed_u32::bit_and(a1130, a1131)
                            };
                            let a1133 = 0u32;
                            if equal(a1132, a1133) {
                                let a1136 = {
                                    let a1134 = v0;
                                    let a1135 = 1u32;
                                    fixed_u32::bit_and(a1134, a1135)
                                };
                                let a1137 = 0u32;
                                if equal(a1136, a1137) {
                                    Bytes::lit(&[0x64])
                                } else {
                                    Bytes::lit(&[0x65])
                                }
                            } else {
                                let a1140 = {
                                    let a1138 = v0;
                                    let a1139 = 1u32;
                                    fixed_u32::bit_and(a1138, a1139)
                                };
                                let a1141 = 0u32;
                                if equal(a1140, a1141) {
                                    Bytes::lit(&[0x66])
                                } else {
                                    Bytes::lit(&[0x67])
                                }
                            }
                        }
                    } else {
                        let a1144 = {
                            let a1142 = v0;
                            let a1143 = 4u32;
                            fixed_u32::bit_and(a1142, a1143)
                        };
                        let a1145 = 0u32;
                        if equal(a1144, a1145) {
                            let a1148 = {
                                let a1146 = v0;
                                let a1147 = 2u32;
                                fixed_u32::bit_and(a1146, a1147)
                            };
                            let a1149 = 0u32;
                            if equal(a1148, a1149) {
                                let a1152 = {
                                    let a1150 = v0;
                                    let a1151 = 1u32;
                                    fixed_u32::bit_and(a1150, a1151)
                                };
                                let a1153 = 0u32;
                                if equal(a1152, a1153) {
                                    Bytes::lit(&[0x68])
                                } else {
                                    Bytes::lit(&[0x69])
                                }
                            } else {
                                let a1156 = {
                                    let a1154 = v0;
                                    let a1155 = 1u32;
                                    fixed_u32::bit_and(a1154, a1155)
                                };
                                let a1157 = 0u32;
                                if equal(a1156, a1157) {
                                    Bytes::lit(&[0x6a])
                                } else {
                                    Bytes::lit(&[0x6b])
                                }
                            }
                        } else {
                            let a1160 = {
                                let a1158 = v0;
                                let a1159 = 2u32;
                                fixed_u32::bit_and(a1158, a1159)
                            };
                            let a1161 = 0u32;
                            if equal(a1160, a1161) {
                                let a1164 = {
                                    let a1162 = v0;
                                    let a1163 = 1u32;
                                    fixed_u32::bit_and(a1162, a1163)
                                };
                                let a1165 = 0u32;
                                if equal(a1164, a1165) {
                                    Bytes::lit(&[0x6c])
                                } else {
                                    Bytes::lit(&[0x6d])
                                }
                            } else {
                                let a1168 = {
                                    let a1166 = v0;
                                    let a1167 = 1u32;
                                    fixed_u32::bit_and(a1166, a1167)
                                };
                                let a1169 = 0u32;
                                if equal(a1168, a1169) {
                                    Bytes::lit(&[0x6e])
                                } else {
                                    Bytes::lit(&[0x6f])
                                }
                            }
                        }
                    }
                } else {
                    let a1172 = {
                        let a1170 = v0;
                        let a1171 = 8u32;
                        fixed_u32::bit_and(a1170, a1171)
                    };
                    let a1173 = 0u32;
                    if equal(a1172, a1173) {
                        let a1176 = {
                            let a1174 = v0;
                            let a1175 = 4u32;
                            fixed_u32::bit_and(a1174, a1175)
                        };
                        let a1177 = 0u32;
                        if equal(a1176, a1177) {
                            let a1180 = {
                                let a1178 = v0;
                                let a1179 = 2u32;
                                fixed_u32::bit_and(a1178, a1179)
                            };
                            let a1181 = 0u32;
                            if equal(a1180, a1181) {
                                let a1184 = {
                                    let a1182 = v0;
                                    let a1183 = 1u32;
                                    fixed_u32::bit_and(a1182, a1183)
                                };
                                let a1185 = 0u32;
                                if equal(a1184, a1185) {
                                    Bytes::lit(&[0x70])
                                } else {
                                    Bytes::lit(&[0x71])
                                }
                            } else {
                                let a1188 = {
                                    let a1186 = v0;
                                    let a1187 = 1u32;
                                    fixed_u32::bit_and(a1186, a1187)
                                };
                                let a1189 = 0u32;
                                if equal(a1188, a1189) {
                                    Bytes::lit(&[0x72])
                                } else {
                                    Bytes::lit(&[0x73])
                                }
                            }
                        } else {
                            let a1192 = {
                                let a1190 = v0;
                                let a1191 = 2u32;
                                fixed_u32::bit_and(a1190, a1191)
                            };
                            let a1193 = 0u32;
                            if equal(a1192, a1193) {
                                let a1196 = {
                                    let a1194 = v0;
                                    let a1195 = 1u32;
                                    fixed_u32::bit_and(a1194, a1195)
                                };
                                let a1197 = 0u32;
                                if equal(a1196, a1197) {
                                    Bytes::lit(&[0x74])
                                } else {
                                    Bytes::lit(&[0x75])
                                }
                            } else {
                                let a1200 = {
                                    let a1198 = v0;
                                    let a1199 = 1u32;
                                    fixed_u32::bit_and(a1198, a1199)
                                };
                                let a1201 = 0u32;
                                if equal(a1200, a1201) {
                                    Bytes::lit(&[0x76])
                                } else {
                                    Bytes::lit(&[0x77])
                                }
                            }
                        }
                    } else {
                        let a1204 = {
                            let a1202 = v0;
                            let a1203 = 4u32;
                            fixed_u32::bit_and(a1202, a1203)
                        };
                        let a1205 = 0u32;
                        if equal(a1204, a1205) {
                            let a1208 = {
                                let a1206 = v0;
                                let a1207 = 2u32;
                                fixed_u32::bit_and(a1206, a1207)
                            };
                            let a1209 = 0u32;
                            if equal(a1208, a1209) {
                                let a1212 = {
                                    let a1210 = v0;
                                    let a1211 = 1u32;
                                    fixed_u32::bit_and(a1210, a1211)
                                };
                                let a1213 = 0u32;
                                if equal(a1212, a1213) {
                                    Bytes::lit(&[0x78])
                                } else {
                                    Bytes::lit(&[0x79])
                                }
                            } else {
                                let a1216 = {
                                    let a1214 = v0;
                                    let a1215 = 1u32;
                                    fixed_u32::bit_and(a1214, a1215)
                                };
                                let a1217 = 0u32;
                                if equal(a1216, a1217) {
                                    Bytes::lit(&[0x7a])
                                } else {
                                    Bytes::lit(&[0x7b])
                                }
                            }
                        } else {
                            let a1220 = {
                                let a1218 = v0;
                                let a1219 = 2u32;
                                fixed_u32::bit_and(a1218, a1219)
                            };
                            let a1221 = 0u32;
                            if equal(a1220, a1221) {
                                let a1224 = {
                                    let a1222 = v0;
                                    let a1223 = 1u32;
                                    fixed_u32::bit_and(a1222, a1223)
                                };
                                let a1225 = 0u32;
                                if equal(a1224, a1225) {
                                    Bytes::lit(&[0x7c])
                                } else {
                                    Bytes::lit(&[0x7d])
                                }
                            } else {
                                let a1228 = {
                                    let a1226 = v0;
                                    let a1227 = 1u32;
                                    fixed_u32::bit_and(a1226, a1227)
                                };
                                let a1229 = 0u32;
                                if equal(a1228, a1229) {
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
        let a1232 = {
            let a1230 = v0;
            let a1231 = 64u32;
            fixed_u32::bit_and(a1230, a1231)
        };
        let a1233 = 0u32;
        if equal(a1232, a1233) {
            let a1236 = {
                let a1234 = v0;
                let a1235 = 32u32;
                fixed_u32::bit_and(a1234, a1235)
            };
            let a1237 = 0u32;
            if equal(a1236, a1237) {
                let a1240 = {
                    let a1238 = v0;
                    let a1239 = 16u32;
                    fixed_u32::bit_and(a1238, a1239)
                };
                let a1241 = 0u32;
                if equal(a1240, a1241) {
                    let a1244 = {
                        let a1242 = v0;
                        let a1243 = 8u32;
                        fixed_u32::bit_and(a1242, a1243)
                    };
                    let a1245 = 0u32;
                    if equal(a1244, a1245) {
                        let a1248 = {
                            let a1246 = v0;
                            let a1247 = 4u32;
                            fixed_u32::bit_and(a1246, a1247)
                        };
                        let a1249 = 0u32;
                        if equal(a1248, a1249) {
                            let a1252 = {
                                let a1250 = v0;
                                let a1251 = 2u32;
                                fixed_u32::bit_and(a1250, a1251)
                            };
                            let a1253 = 0u32;
                            if equal(a1252, a1253) {
                                let a1256 = {
                                    let a1254 = v0;
                                    let a1255 = 1u32;
                                    fixed_u32::bit_and(a1254, a1255)
                                };
                                let a1257 = 0u32;
                                if equal(a1256, a1257) {
                                    Bytes::lit(&[0x80])
                                } else {
                                    Bytes::lit(&[0x81])
                                }
                            } else {
                                let a1260 = {
                                    let a1258 = v0;
                                    let a1259 = 1u32;
                                    fixed_u32::bit_and(a1258, a1259)
                                };
                                let a1261 = 0u32;
                                if equal(a1260, a1261) {
                                    Bytes::lit(&[0x82])
                                } else {
                                    Bytes::lit(&[0x83])
                                }
                            }
                        } else {
                            let a1264 = {
                                let a1262 = v0;
                                let a1263 = 2u32;
                                fixed_u32::bit_and(a1262, a1263)
                            };
                            let a1265 = 0u32;
                            if equal(a1264, a1265) {
                                let a1268 = {
                                    let a1266 = v0;
                                    let a1267 = 1u32;
                                    fixed_u32::bit_and(a1266, a1267)
                                };
                                let a1269 = 0u32;
                                if equal(a1268, a1269) {
                                    Bytes::lit(&[0x84])
                                } else {
                                    Bytes::lit(&[0x85])
                                }
                            } else {
                                let a1272 = {
                                    let a1270 = v0;
                                    let a1271 = 1u32;
                                    fixed_u32::bit_and(a1270, a1271)
                                };
                                let a1273 = 0u32;
                                if equal(a1272, a1273) {
                                    Bytes::lit(&[0x86])
                                } else {
                                    Bytes::lit(&[0x87])
                                }
                            }
                        }
                    } else {
                        let a1276 = {
                            let a1274 = v0;
                            let a1275 = 4u32;
                            fixed_u32::bit_and(a1274, a1275)
                        };
                        let a1277 = 0u32;
                        if equal(a1276, a1277) {
                            let a1280 = {
                                let a1278 = v0;
                                let a1279 = 2u32;
                                fixed_u32::bit_and(a1278, a1279)
                            };
                            let a1281 = 0u32;
                            if equal(a1280, a1281) {
                                let a1284 = {
                                    let a1282 = v0;
                                    let a1283 = 1u32;
                                    fixed_u32::bit_and(a1282, a1283)
                                };
                                let a1285 = 0u32;
                                if equal(a1284, a1285) {
                                    Bytes::lit(&[0x88])
                                } else {
                                    Bytes::lit(&[0x89])
                                }
                            } else {
                                let a1288 = {
                                    let a1286 = v0;
                                    let a1287 = 1u32;
                                    fixed_u32::bit_and(a1286, a1287)
                                };
                                let a1289 = 0u32;
                                if equal(a1288, a1289) {
                                    Bytes::lit(&[0x8a])
                                } else {
                                    Bytes::lit(&[0x8b])
                                }
                            }
                        } else {
                            let a1292 = {
                                let a1290 = v0;
                                let a1291 = 2u32;
                                fixed_u32::bit_and(a1290, a1291)
                            };
                            let a1293 = 0u32;
                            if equal(a1292, a1293) {
                                let a1296 = {
                                    let a1294 = v0;
                                    let a1295 = 1u32;
                                    fixed_u32::bit_and(a1294, a1295)
                                };
                                let a1297 = 0u32;
                                if equal(a1296, a1297) {
                                    Bytes::lit(&[0x8c])
                                } else {
                                    Bytes::lit(&[0x8d])
                                }
                            } else {
                                let a1300 = {
                                    let a1298 = v0;
                                    let a1299 = 1u32;
                                    fixed_u32::bit_and(a1298, a1299)
                                };
                                let a1301 = 0u32;
                                if equal(a1300, a1301) {
                                    Bytes::lit(&[0x8e])
                                } else {
                                    Bytes::lit(&[0x8f])
                                }
                            }
                        }
                    }
                } else {
                    let a1304 = {
                        let a1302 = v0;
                        let a1303 = 8u32;
                        fixed_u32::bit_and(a1302, a1303)
                    };
                    let a1305 = 0u32;
                    if equal(a1304, a1305) {
                        let a1308 = {
                            let a1306 = v0;
                            let a1307 = 4u32;
                            fixed_u32::bit_and(a1306, a1307)
                        };
                        let a1309 = 0u32;
                        if equal(a1308, a1309) {
                            let a1312 = {
                                let a1310 = v0;
                                let a1311 = 2u32;
                                fixed_u32::bit_and(a1310, a1311)
                            };
                            let a1313 = 0u32;
                            if equal(a1312, a1313) {
                                let a1316 = {
                                    let a1314 = v0;
                                    let a1315 = 1u32;
                                    fixed_u32::bit_and(a1314, a1315)
                                };
                                let a1317 = 0u32;
                                if equal(a1316, a1317) {
                                    Bytes::lit(&[0x90])
                                } else {
                                    Bytes::lit(&[0x91])
                                }
                            } else {
                                let a1320 = {
                                    let a1318 = v0;
                                    let a1319 = 1u32;
                                    fixed_u32::bit_and(a1318, a1319)
                                };
                                let a1321 = 0u32;
                                if equal(a1320, a1321) {
                                    Bytes::lit(&[0x92])
                                } else {
                                    Bytes::lit(&[0x93])
                                }
                            }
                        } else {
                            let a1324 = {
                                let a1322 = v0;
                                let a1323 = 2u32;
                                fixed_u32::bit_and(a1322, a1323)
                            };
                            let a1325 = 0u32;
                            if equal(a1324, a1325) {
                                let a1328 = {
                                    let a1326 = v0;
                                    let a1327 = 1u32;
                                    fixed_u32::bit_and(a1326, a1327)
                                };
                                let a1329 = 0u32;
                                if equal(a1328, a1329) {
                                    Bytes::lit(&[0x94])
                                } else {
                                    Bytes::lit(&[0x95])
                                }
                            } else {
                                let a1332 = {
                                    let a1330 = v0;
                                    let a1331 = 1u32;
                                    fixed_u32::bit_and(a1330, a1331)
                                };
                                let a1333 = 0u32;
                                if equal(a1332, a1333) {
                                    Bytes::lit(&[0x96])
                                } else {
                                    Bytes::lit(&[0x97])
                                }
                            }
                        }
                    } else {
                        let a1336 = {
                            let a1334 = v0;
                            let a1335 = 4u32;
                            fixed_u32::bit_and(a1334, a1335)
                        };
                        let a1337 = 0u32;
                        if equal(a1336, a1337) {
                            let a1340 = {
                                let a1338 = v0;
                                let a1339 = 2u32;
                                fixed_u32::bit_and(a1338, a1339)
                            };
                            let a1341 = 0u32;
                            if equal(a1340, a1341) {
                                let a1344 = {
                                    let a1342 = v0;
                                    let a1343 = 1u32;
                                    fixed_u32::bit_and(a1342, a1343)
                                };
                                let a1345 = 0u32;
                                if equal(a1344, a1345) {
                                    Bytes::lit(&[0x98])
                                } else {
                                    Bytes::lit(&[0x99])
                                }
                            } else {
                                let a1348 = {
                                    let a1346 = v0;
                                    let a1347 = 1u32;
                                    fixed_u32::bit_and(a1346, a1347)
                                };
                                let a1349 = 0u32;
                                if equal(a1348, a1349) {
                                    Bytes::lit(&[0x9a])
                                } else {
                                    Bytes::lit(&[0x9b])
                                }
                            }
                        } else {
                            let a1352 = {
                                let a1350 = v0;
                                let a1351 = 2u32;
                                fixed_u32::bit_and(a1350, a1351)
                            };
                            let a1353 = 0u32;
                            if equal(a1352, a1353) {
                                let a1356 = {
                                    let a1354 = v0;
                                    let a1355 = 1u32;
                                    fixed_u32::bit_and(a1354, a1355)
                                };
                                let a1357 = 0u32;
                                if equal(a1356, a1357) {
                                    Bytes::lit(&[0x9c])
                                } else {
                                    Bytes::lit(&[0x9d])
                                }
                            } else {
                                let a1360 = {
                                    let a1358 = v0;
                                    let a1359 = 1u32;
                                    fixed_u32::bit_and(a1358, a1359)
                                };
                                let a1361 = 0u32;
                                if equal(a1360, a1361) {
                                    Bytes::lit(&[0x9e])
                                } else {
                                    Bytes::lit(&[0x9f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a1364 = {
                    let a1362 = v0;
                    let a1363 = 16u32;
                    fixed_u32::bit_and(a1362, a1363)
                };
                let a1365 = 0u32;
                if equal(a1364, a1365) {
                    let a1368 = {
                        let a1366 = v0;
                        let a1367 = 8u32;
                        fixed_u32::bit_and(a1366, a1367)
                    };
                    let a1369 = 0u32;
                    if equal(a1368, a1369) {
                        let a1372 = {
                            let a1370 = v0;
                            let a1371 = 4u32;
                            fixed_u32::bit_and(a1370, a1371)
                        };
                        let a1373 = 0u32;
                        if equal(a1372, a1373) {
                            let a1376 = {
                                let a1374 = v0;
                                let a1375 = 2u32;
                                fixed_u32::bit_and(a1374, a1375)
                            };
                            let a1377 = 0u32;
                            if equal(a1376, a1377) {
                                let a1380 = {
                                    let a1378 = v0;
                                    let a1379 = 1u32;
                                    fixed_u32::bit_and(a1378, a1379)
                                };
                                let a1381 = 0u32;
                                if equal(a1380, a1381) {
                                    Bytes::lit(&[0xa0])
                                } else {
                                    Bytes::lit(&[0xa1])
                                }
                            } else {
                                let a1384 = {
                                    let a1382 = v0;
                                    let a1383 = 1u32;
                                    fixed_u32::bit_and(a1382, a1383)
                                };
                                let a1385 = 0u32;
                                if equal(a1384, a1385) {
                                    Bytes::lit(&[0xa2])
                                } else {
                                    Bytes::lit(&[0xa3])
                                }
                            }
                        } else {
                            let a1388 = {
                                let a1386 = v0;
                                let a1387 = 2u32;
                                fixed_u32::bit_and(a1386, a1387)
                            };
                            let a1389 = 0u32;
                            if equal(a1388, a1389) {
                                let a1392 = {
                                    let a1390 = v0;
                                    let a1391 = 1u32;
                                    fixed_u32::bit_and(a1390, a1391)
                                };
                                let a1393 = 0u32;
                                if equal(a1392, a1393) {
                                    Bytes::lit(&[0xa4])
                                } else {
                                    Bytes::lit(&[0xa5])
                                }
                            } else {
                                let a1396 = {
                                    let a1394 = v0;
                                    let a1395 = 1u32;
                                    fixed_u32::bit_and(a1394, a1395)
                                };
                                let a1397 = 0u32;
                                if equal(a1396, a1397) {
                                    Bytes::lit(&[0xa6])
                                } else {
                                    Bytes::lit(&[0xa7])
                                }
                            }
                        }
                    } else {
                        let a1400 = {
                            let a1398 = v0;
                            let a1399 = 4u32;
                            fixed_u32::bit_and(a1398, a1399)
                        };
                        let a1401 = 0u32;
                        if equal(a1400, a1401) {
                            let a1404 = {
                                let a1402 = v0;
                                let a1403 = 2u32;
                                fixed_u32::bit_and(a1402, a1403)
                            };
                            let a1405 = 0u32;
                            if equal(a1404, a1405) {
                                let a1408 = {
                                    let a1406 = v0;
                                    let a1407 = 1u32;
                                    fixed_u32::bit_and(a1406, a1407)
                                };
                                let a1409 = 0u32;
                                if equal(a1408, a1409) {
                                    Bytes::lit(&[0xa8])
                                } else {
                                    Bytes::lit(&[0xa9])
                                }
                            } else {
                                let a1412 = {
                                    let a1410 = v0;
                                    let a1411 = 1u32;
                                    fixed_u32::bit_and(a1410, a1411)
                                };
                                let a1413 = 0u32;
                                if equal(a1412, a1413) {
                                    Bytes::lit(&[0xaa])
                                } else {
                                    Bytes::lit(&[0xab])
                                }
                            }
                        } else {
                            let a1416 = {
                                let a1414 = v0;
                                let a1415 = 2u32;
                                fixed_u32::bit_and(a1414, a1415)
                            };
                            let a1417 = 0u32;
                            if equal(a1416, a1417) {
                                let a1420 = {
                                    let a1418 = v0;
                                    let a1419 = 1u32;
                                    fixed_u32::bit_and(a1418, a1419)
                                };
                                let a1421 = 0u32;
                                if equal(a1420, a1421) {
                                    Bytes::lit(&[0xac])
                                } else {
                                    Bytes::lit(&[0xad])
                                }
                            } else {
                                let a1424 = {
                                    let a1422 = v0;
                                    let a1423 = 1u32;
                                    fixed_u32::bit_and(a1422, a1423)
                                };
                                let a1425 = 0u32;
                                if equal(a1424, a1425) {
                                    Bytes::lit(&[0xae])
                                } else {
                                    Bytes::lit(&[0xaf])
                                }
                            }
                        }
                    }
                } else {
                    let a1428 = {
                        let a1426 = v0;
                        let a1427 = 8u32;
                        fixed_u32::bit_and(a1426, a1427)
                    };
                    let a1429 = 0u32;
                    if equal(a1428, a1429) {
                        let a1432 = {
                            let a1430 = v0;
                            let a1431 = 4u32;
                            fixed_u32::bit_and(a1430, a1431)
                        };
                        let a1433 = 0u32;
                        if equal(a1432, a1433) {
                            let a1436 = {
                                let a1434 = v0;
                                let a1435 = 2u32;
                                fixed_u32::bit_and(a1434, a1435)
                            };
                            let a1437 = 0u32;
                            if equal(a1436, a1437) {
                                let a1440 = {
                                    let a1438 = v0;
                                    let a1439 = 1u32;
                                    fixed_u32::bit_and(a1438, a1439)
                                };
                                let a1441 = 0u32;
                                if equal(a1440, a1441) {
                                    Bytes::lit(&[0xb0])
                                } else {
                                    Bytes::lit(&[0xb1])
                                }
                            } else {
                                let a1444 = {
                                    let a1442 = v0;
                                    let a1443 = 1u32;
                                    fixed_u32::bit_and(a1442, a1443)
                                };
                                let a1445 = 0u32;
                                if equal(a1444, a1445) {
                                    Bytes::lit(&[0xb2])
                                } else {
                                    Bytes::lit(&[0xb3])
                                }
                            }
                        } else {
                            let a1448 = {
                                let a1446 = v0;
                                let a1447 = 2u32;
                                fixed_u32::bit_and(a1446, a1447)
                            };
                            let a1449 = 0u32;
                            if equal(a1448, a1449) {
                                let a1452 = {
                                    let a1450 = v0;
                                    let a1451 = 1u32;
                                    fixed_u32::bit_and(a1450, a1451)
                                };
                                let a1453 = 0u32;
                                if equal(a1452, a1453) {
                                    Bytes::lit(&[0xb4])
                                } else {
                                    Bytes::lit(&[0xb5])
                                }
                            } else {
                                let a1456 = {
                                    let a1454 = v0;
                                    let a1455 = 1u32;
                                    fixed_u32::bit_and(a1454, a1455)
                                };
                                let a1457 = 0u32;
                                if equal(a1456, a1457) {
                                    Bytes::lit(&[0xb6])
                                } else {
                                    Bytes::lit(&[0xb7])
                                }
                            }
                        }
                    } else {
                        let a1460 = {
                            let a1458 = v0;
                            let a1459 = 4u32;
                            fixed_u32::bit_and(a1458, a1459)
                        };
                        let a1461 = 0u32;
                        if equal(a1460, a1461) {
                            let a1464 = {
                                let a1462 = v0;
                                let a1463 = 2u32;
                                fixed_u32::bit_and(a1462, a1463)
                            };
                            let a1465 = 0u32;
                            if equal(a1464, a1465) {
                                let a1468 = {
                                    let a1466 = v0;
                                    let a1467 = 1u32;
                                    fixed_u32::bit_and(a1466, a1467)
                                };
                                let a1469 = 0u32;
                                if equal(a1468, a1469) {
                                    Bytes::lit(&[0xb8])
                                } else {
                                    Bytes::lit(&[0xb9])
                                }
                            } else {
                                let a1472 = {
                                    let a1470 = v0;
                                    let a1471 = 1u32;
                                    fixed_u32::bit_and(a1470, a1471)
                                };
                                let a1473 = 0u32;
                                if equal(a1472, a1473) {
                                    Bytes::lit(&[0xba])
                                } else {
                                    Bytes::lit(&[0xbb])
                                }
                            }
                        } else {
                            let a1476 = {
                                let a1474 = v0;
                                let a1475 = 2u32;
                                fixed_u32::bit_and(a1474, a1475)
                            };
                            let a1477 = 0u32;
                            if equal(a1476, a1477) {
                                let a1480 = {
                                    let a1478 = v0;
                                    let a1479 = 1u32;
                                    fixed_u32::bit_and(a1478, a1479)
                                };
                                let a1481 = 0u32;
                                if equal(a1480, a1481) {
                                    Bytes::lit(&[0xbc])
                                } else {
                                    Bytes::lit(&[0xbd])
                                }
                            } else {
                                let a1484 = {
                                    let a1482 = v0;
                                    let a1483 = 1u32;
                                    fixed_u32::bit_and(a1482, a1483)
                                };
                                let a1485 = 0u32;
                                if equal(a1484, a1485) {
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
            let a1488 = {
                let a1486 = v0;
                let a1487 = 32u32;
                fixed_u32::bit_and(a1486, a1487)
            };
            let a1489 = 0u32;
            if equal(a1488, a1489) {
                let a1492 = {
                    let a1490 = v0;
                    let a1491 = 16u32;
                    fixed_u32::bit_and(a1490, a1491)
                };
                let a1493 = 0u32;
                if equal(a1492, a1493) {
                    let a1496 = {
                        let a1494 = v0;
                        let a1495 = 8u32;
                        fixed_u32::bit_and(a1494, a1495)
                    };
                    let a1497 = 0u32;
                    if equal(a1496, a1497) {
                        let a1500 = {
                            let a1498 = v0;
                            let a1499 = 4u32;
                            fixed_u32::bit_and(a1498, a1499)
                        };
                        let a1501 = 0u32;
                        if equal(a1500, a1501) {
                            let a1504 = {
                                let a1502 = v0;
                                let a1503 = 2u32;
                                fixed_u32::bit_and(a1502, a1503)
                            };
                            let a1505 = 0u32;
                            if equal(a1504, a1505) {
                                let a1508 = {
                                    let a1506 = v0;
                                    let a1507 = 1u32;
                                    fixed_u32::bit_and(a1506, a1507)
                                };
                                let a1509 = 0u32;
                                if equal(a1508, a1509) {
                                    Bytes::lit(&[0xc0])
                                } else {
                                    Bytes::lit(&[0xc1])
                                }
                            } else {
                                let a1512 = {
                                    let a1510 = v0;
                                    let a1511 = 1u32;
                                    fixed_u32::bit_and(a1510, a1511)
                                };
                                let a1513 = 0u32;
                                if equal(a1512, a1513) {
                                    Bytes::lit(&[0xc2])
                                } else {
                                    Bytes::lit(&[0xc3])
                                }
                            }
                        } else {
                            let a1516 = {
                                let a1514 = v0;
                                let a1515 = 2u32;
                                fixed_u32::bit_and(a1514, a1515)
                            };
                            let a1517 = 0u32;
                            if equal(a1516, a1517) {
                                let a1520 = {
                                    let a1518 = v0;
                                    let a1519 = 1u32;
                                    fixed_u32::bit_and(a1518, a1519)
                                };
                                let a1521 = 0u32;
                                if equal(a1520, a1521) {
                                    Bytes::lit(&[0xc4])
                                } else {
                                    Bytes::lit(&[0xc5])
                                }
                            } else {
                                let a1524 = {
                                    let a1522 = v0;
                                    let a1523 = 1u32;
                                    fixed_u32::bit_and(a1522, a1523)
                                };
                                let a1525 = 0u32;
                                if equal(a1524, a1525) {
                                    Bytes::lit(&[0xc6])
                                } else {
                                    Bytes::lit(&[0xc7])
                                }
                            }
                        }
                    } else {
                        let a1528 = {
                            let a1526 = v0;
                            let a1527 = 4u32;
                            fixed_u32::bit_and(a1526, a1527)
                        };
                        let a1529 = 0u32;
                        if equal(a1528, a1529) {
                            let a1532 = {
                                let a1530 = v0;
                                let a1531 = 2u32;
                                fixed_u32::bit_and(a1530, a1531)
                            };
                            let a1533 = 0u32;
                            if equal(a1532, a1533) {
                                let a1536 = {
                                    let a1534 = v0;
                                    let a1535 = 1u32;
                                    fixed_u32::bit_and(a1534, a1535)
                                };
                                let a1537 = 0u32;
                                if equal(a1536, a1537) {
                                    Bytes::lit(&[0xc8])
                                } else {
                                    Bytes::lit(&[0xc9])
                                }
                            } else {
                                let a1540 = {
                                    let a1538 = v0;
                                    let a1539 = 1u32;
                                    fixed_u32::bit_and(a1538, a1539)
                                };
                                let a1541 = 0u32;
                                if equal(a1540, a1541) {
                                    Bytes::lit(&[0xca])
                                } else {
                                    Bytes::lit(&[0xcb])
                                }
                            }
                        } else {
                            let a1544 = {
                                let a1542 = v0;
                                let a1543 = 2u32;
                                fixed_u32::bit_and(a1542, a1543)
                            };
                            let a1545 = 0u32;
                            if equal(a1544, a1545) {
                                let a1548 = {
                                    let a1546 = v0;
                                    let a1547 = 1u32;
                                    fixed_u32::bit_and(a1546, a1547)
                                };
                                let a1549 = 0u32;
                                if equal(a1548, a1549) {
                                    Bytes::lit(&[0xcc])
                                } else {
                                    Bytes::lit(&[0xcd])
                                }
                            } else {
                                let a1552 = {
                                    let a1550 = v0;
                                    let a1551 = 1u32;
                                    fixed_u32::bit_and(a1550, a1551)
                                };
                                let a1553 = 0u32;
                                if equal(a1552, a1553) {
                                    Bytes::lit(&[0xce])
                                } else {
                                    Bytes::lit(&[0xcf])
                                }
                            }
                        }
                    }
                } else {
                    let a1556 = {
                        let a1554 = v0;
                        let a1555 = 8u32;
                        fixed_u32::bit_and(a1554, a1555)
                    };
                    let a1557 = 0u32;
                    if equal(a1556, a1557) {
                        let a1560 = {
                            let a1558 = v0;
                            let a1559 = 4u32;
                            fixed_u32::bit_and(a1558, a1559)
                        };
                        let a1561 = 0u32;
                        if equal(a1560, a1561) {
                            let a1564 = {
                                let a1562 = v0;
                                let a1563 = 2u32;
                                fixed_u32::bit_and(a1562, a1563)
                            };
                            let a1565 = 0u32;
                            if equal(a1564, a1565) {
                                let a1568 = {
                                    let a1566 = v0;
                                    let a1567 = 1u32;
                                    fixed_u32::bit_and(a1566, a1567)
                                };
                                let a1569 = 0u32;
                                if equal(a1568, a1569) {
                                    Bytes::lit(&[0xd0])
                                } else {
                                    Bytes::lit(&[0xd1])
                                }
                            } else {
                                let a1572 = {
                                    let a1570 = v0;
                                    let a1571 = 1u32;
                                    fixed_u32::bit_and(a1570, a1571)
                                };
                                let a1573 = 0u32;
                                if equal(a1572, a1573) {
                                    Bytes::lit(&[0xd2])
                                } else {
                                    Bytes::lit(&[0xd3])
                                }
                            }
                        } else {
                            let a1576 = {
                                let a1574 = v0;
                                let a1575 = 2u32;
                                fixed_u32::bit_and(a1574, a1575)
                            };
                            let a1577 = 0u32;
                            if equal(a1576, a1577) {
                                let a1580 = {
                                    let a1578 = v0;
                                    let a1579 = 1u32;
                                    fixed_u32::bit_and(a1578, a1579)
                                };
                                let a1581 = 0u32;
                                if equal(a1580, a1581) {
                                    Bytes::lit(&[0xd4])
                                } else {
                                    Bytes::lit(&[0xd5])
                                }
                            } else {
                                let a1584 = {
                                    let a1582 = v0;
                                    let a1583 = 1u32;
                                    fixed_u32::bit_and(a1582, a1583)
                                };
                                let a1585 = 0u32;
                                if equal(a1584, a1585) {
                                    Bytes::lit(&[0xd6])
                                } else {
                                    Bytes::lit(&[0xd7])
                                }
                            }
                        }
                    } else {
                        let a1588 = {
                            let a1586 = v0;
                            let a1587 = 4u32;
                            fixed_u32::bit_and(a1586, a1587)
                        };
                        let a1589 = 0u32;
                        if equal(a1588, a1589) {
                            let a1592 = {
                                let a1590 = v0;
                                let a1591 = 2u32;
                                fixed_u32::bit_and(a1590, a1591)
                            };
                            let a1593 = 0u32;
                            if equal(a1592, a1593) {
                                let a1596 = {
                                    let a1594 = v0;
                                    let a1595 = 1u32;
                                    fixed_u32::bit_and(a1594, a1595)
                                };
                                let a1597 = 0u32;
                                if equal(a1596, a1597) {
                                    Bytes::lit(&[0xd8])
                                } else {
                                    Bytes::lit(&[0xd9])
                                }
                            } else {
                                let a1600 = {
                                    let a1598 = v0;
                                    let a1599 = 1u32;
                                    fixed_u32::bit_and(a1598, a1599)
                                };
                                let a1601 = 0u32;
                                if equal(a1600, a1601) {
                                    Bytes::lit(&[0xda])
                                } else {
                                    Bytes::lit(&[0xdb])
                                }
                            }
                        } else {
                            let a1604 = {
                                let a1602 = v0;
                                let a1603 = 2u32;
                                fixed_u32::bit_and(a1602, a1603)
                            };
                            let a1605 = 0u32;
                            if equal(a1604, a1605) {
                                let a1608 = {
                                    let a1606 = v0;
                                    let a1607 = 1u32;
                                    fixed_u32::bit_and(a1606, a1607)
                                };
                                let a1609 = 0u32;
                                if equal(a1608, a1609) {
                                    Bytes::lit(&[0xdc])
                                } else {
                                    Bytes::lit(&[0xdd])
                                }
                            } else {
                                let a1612 = {
                                    let a1610 = v0;
                                    let a1611 = 1u32;
                                    fixed_u32::bit_and(a1610, a1611)
                                };
                                let a1613 = 0u32;
                                if equal(a1612, a1613) {
                                    Bytes::lit(&[0xde])
                                } else {
                                    Bytes::lit(&[0xdf])
                                }
                            }
                        }
                    }
                }
            } else {
                let a1616 = {
                    let a1614 = v0;
                    let a1615 = 16u32;
                    fixed_u32::bit_and(a1614, a1615)
                };
                let a1617 = 0u32;
                if equal(a1616, a1617) {
                    let a1620 = {
                        let a1618 = v0;
                        let a1619 = 8u32;
                        fixed_u32::bit_and(a1618, a1619)
                    };
                    let a1621 = 0u32;
                    if equal(a1620, a1621) {
                        let a1624 = {
                            let a1622 = v0;
                            let a1623 = 4u32;
                            fixed_u32::bit_and(a1622, a1623)
                        };
                        let a1625 = 0u32;
                        if equal(a1624, a1625) {
                            let a1628 = {
                                let a1626 = v0;
                                let a1627 = 2u32;
                                fixed_u32::bit_and(a1626, a1627)
                            };
                            let a1629 = 0u32;
                            if equal(a1628, a1629) {
                                let a1632 = {
                                    let a1630 = v0;
                                    let a1631 = 1u32;
                                    fixed_u32::bit_and(a1630, a1631)
                                };
                                let a1633 = 0u32;
                                if equal(a1632, a1633) {
                                    Bytes::lit(&[0xe0])
                                } else {
                                    Bytes::lit(&[0xe1])
                                }
                            } else {
                                let a1636 = {
                                    let a1634 = v0;
                                    let a1635 = 1u32;
                                    fixed_u32::bit_and(a1634, a1635)
                                };
                                let a1637 = 0u32;
                                if equal(a1636, a1637) {
                                    Bytes::lit(&[0xe2])
                                } else {
                                    Bytes::lit(&[0xe3])
                                }
                            }
                        } else {
                            let a1640 = {
                                let a1638 = v0;
                                let a1639 = 2u32;
                                fixed_u32::bit_and(a1638, a1639)
                            };
                            let a1641 = 0u32;
                            if equal(a1640, a1641) {
                                let a1644 = {
                                    let a1642 = v0;
                                    let a1643 = 1u32;
                                    fixed_u32::bit_and(a1642, a1643)
                                };
                                let a1645 = 0u32;
                                if equal(a1644, a1645) {
                                    Bytes::lit(&[0xe4])
                                } else {
                                    Bytes::lit(&[0xe5])
                                }
                            } else {
                                let a1648 = {
                                    let a1646 = v0;
                                    let a1647 = 1u32;
                                    fixed_u32::bit_and(a1646, a1647)
                                };
                                let a1649 = 0u32;
                                if equal(a1648, a1649) {
                                    Bytes::lit(&[0xe6])
                                } else {
                                    Bytes::lit(&[0xe7])
                                }
                            }
                        }
                    } else {
                        let a1652 = {
                            let a1650 = v0;
                            let a1651 = 4u32;
                            fixed_u32::bit_and(a1650, a1651)
                        };
                        let a1653 = 0u32;
                        if equal(a1652, a1653) {
                            let a1656 = {
                                let a1654 = v0;
                                let a1655 = 2u32;
                                fixed_u32::bit_and(a1654, a1655)
                            };
                            let a1657 = 0u32;
                            if equal(a1656, a1657) {
                                let a1660 = {
                                    let a1658 = v0;
                                    let a1659 = 1u32;
                                    fixed_u32::bit_and(a1658, a1659)
                                };
                                let a1661 = 0u32;
                                if equal(a1660, a1661) {
                                    Bytes::lit(&[0xe8])
                                } else {
                                    Bytes::lit(&[0xe9])
                                }
                            } else {
                                let a1664 = {
                                    let a1662 = v0;
                                    let a1663 = 1u32;
                                    fixed_u32::bit_and(a1662, a1663)
                                };
                                let a1665 = 0u32;
                                if equal(a1664, a1665) {
                                    Bytes::lit(&[0xea])
                                } else {
                                    Bytes::lit(&[0xeb])
                                }
                            }
                        } else {
                            let a1668 = {
                                let a1666 = v0;
                                let a1667 = 2u32;
                                fixed_u32::bit_and(a1666, a1667)
                            };
                            let a1669 = 0u32;
                            if equal(a1668, a1669) {
                                let a1672 = {
                                    let a1670 = v0;
                                    let a1671 = 1u32;
                                    fixed_u32::bit_and(a1670, a1671)
                                };
                                let a1673 = 0u32;
                                if equal(a1672, a1673) {
                                    Bytes::lit(&[0xec])
                                } else {
                                    Bytes::lit(&[0xed])
                                }
                            } else {
                                let a1676 = {
                                    let a1674 = v0;
                                    let a1675 = 1u32;
                                    fixed_u32::bit_and(a1674, a1675)
                                };
                                let a1677 = 0u32;
                                if equal(a1676, a1677) {
                                    Bytes::lit(&[0xee])
                                } else {
                                    Bytes::lit(&[0xef])
                                }
                            }
                        }
                    }
                } else {
                    let a1680 = {
                        let a1678 = v0;
                        let a1679 = 8u32;
                        fixed_u32::bit_and(a1678, a1679)
                    };
                    let a1681 = 0u32;
                    if equal(a1680, a1681) {
                        let a1684 = {
                            let a1682 = v0;
                            let a1683 = 4u32;
                            fixed_u32::bit_and(a1682, a1683)
                        };
                        let a1685 = 0u32;
                        if equal(a1684, a1685) {
                            let a1688 = {
                                let a1686 = v0;
                                let a1687 = 2u32;
                                fixed_u32::bit_and(a1686, a1687)
                            };
                            let a1689 = 0u32;
                            if equal(a1688, a1689) {
                                let a1692 = {
                                    let a1690 = v0;
                                    let a1691 = 1u32;
                                    fixed_u32::bit_and(a1690, a1691)
                                };
                                let a1693 = 0u32;
                                if equal(a1692, a1693) {
                                    Bytes::lit(&[0xf0])
                                } else {
                                    Bytes::lit(&[0xf1])
                                }
                            } else {
                                let a1696 = {
                                    let a1694 = v0;
                                    let a1695 = 1u32;
                                    fixed_u32::bit_and(a1694, a1695)
                                };
                                let a1697 = 0u32;
                                if equal(a1696, a1697) {
                                    Bytes::lit(&[0xf2])
                                } else {
                                    Bytes::lit(&[0xf3])
                                }
                            }
                        } else {
                            let a1700 = {
                                let a1698 = v0;
                                let a1699 = 2u32;
                                fixed_u32::bit_and(a1698, a1699)
                            };
                            let a1701 = 0u32;
                            if equal(a1700, a1701) {
                                let a1704 = {
                                    let a1702 = v0;
                                    let a1703 = 1u32;
                                    fixed_u32::bit_and(a1702, a1703)
                                };
                                let a1705 = 0u32;
                                if equal(a1704, a1705) {
                                    Bytes::lit(&[0xf4])
                                } else {
                                    Bytes::lit(&[0xf5])
                                }
                            } else {
                                let a1708 = {
                                    let a1706 = v0;
                                    let a1707 = 1u32;
                                    fixed_u32::bit_and(a1706, a1707)
                                };
                                let a1709 = 0u32;
                                if equal(a1708, a1709) {
                                    Bytes::lit(&[0xf6])
                                } else {
                                    Bytes::lit(&[0xf7])
                                }
                            }
                        }
                    } else {
                        let a1712 = {
                            let a1710 = v0;
                            let a1711 = 4u32;
                            fixed_u32::bit_and(a1710, a1711)
                        };
                        let a1713 = 0u32;
                        if equal(a1712, a1713) {
                            let a1716 = {
                                let a1714 = v0;
                                let a1715 = 2u32;
                                fixed_u32::bit_and(a1714, a1715)
                            };
                            let a1717 = 0u32;
                            if equal(a1716, a1717) {
                                let a1720 = {
                                    let a1718 = v0;
                                    let a1719 = 1u32;
                                    fixed_u32::bit_and(a1718, a1719)
                                };
                                let a1721 = 0u32;
                                if equal(a1720, a1721) {
                                    Bytes::lit(&[0xf8])
                                } else {
                                    Bytes::lit(&[0xf9])
                                }
                            } else {
                                let a1724 = {
                                    let a1722 = v0;
                                    let a1723 = 1u32;
                                    fixed_u32::bit_and(a1722, a1723)
                                };
                                let a1725 = 0u32;
                                if equal(a1724, a1725) {
                                    Bytes::lit(&[0xfa])
                                } else {
                                    Bytes::lit(&[0xfb])
                                }
                            }
                        } else {
                            let a1728 = {
                                let a1726 = v0;
                                let a1727 = 2u32;
                                fixed_u32::bit_and(a1726, a1727)
                            };
                            let a1729 = 0u32;
                            if equal(a1728, a1729) {
                                let a1732 = {
                                    let a1730 = v0;
                                    let a1731 = 1u32;
                                    fixed_u32::bit_and(a1730, a1731)
                                };
                                let a1733 = 0u32;
                                if equal(a1732, a1733) {
                                    Bytes::lit(&[0xfc])
                                } else {
                                    Bytes::lit(&[0xfd])
                                }
                            } else {
                                let a1736 = {
                                    let a1734 = v0;
                                    let a1735 = 1u32;
                                    fixed_u32::bit_and(a1734, a1735)
                                };
                                let a1737 = 0u32;
                                if equal(a1736, a1737) {
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

pub fn f71(v0: u64, v1: u64) -> u32 {
    let (_, h1738) = {
        let (_, h1739) = f92(Fn13::F93, v1, (v0, (1u32, 0u32)));
        h1739
    };
    h1738
}

pub fn f72(v0: Fn10, p1743: List<Bytes>, p1744: List<Adt3>) -> List<Bytes> {
    let m1742 = (p1743, p1744);
    {
        let mut m1742 = m1742;
        loop {
            let step = {
                let (v1, v2) = m1742;
                let m1740 = v2;
                match m1740.uncons() {
                    None => {
                        Err::<(List<Bytes>, List<Adt3>), List<Bytes>>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(List<Bytes>, List<Adt3>), List<Bytes>>(({
                            let c1741: Fn10 = v0.clone();
                            c1741.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m1742 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f73(v0: List<Bytes>, v1: Adt3) -> List<Bytes> {
    let m1745 = v1;
    match m1745 {
        Adt3::C0(v2) => {
            let a1747 = match v2.clone() {
                Adt4::C0(h1746, _, _, _, _, _, _, _, _, _, _, _) => {
                    h1746
                }
            };
            let a1748 = 2u64;
            if nat_eq(a1747, a1748) {
                List::cons(f78(match v2 {
                    Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1749) => {
                        h1749
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

pub fn f74(v0: List<Bytes>, v1: Bytes) -> List<Bytes> {
    List::cons(v1, v0)
}

pub fn f75(v0: Adt4) -> Bytes {
    f2(List::cons(Bytes::lit(&[0x01]), List::cons(f37(match v0.clone() {
        Adt4::C0(h1750, _, _, _, _, _, _, _, _, _, _, _) => {
            h1750
        }
    }), List::cons(f33(match v0.clone() {
        Adt4::C0(_, h1751, _, _, _, _, _, _, _, _, _, _) => {
            h1751
        }
    }), List::cons(f33(match v0.clone() {
        Adt4::C0(_, _, h1752, _, _, _, _, _, _, _, _, _) => {
            h1752
        }
    }), List::cons(f33(match v0.clone() {
        Adt4::C0(_, _, _, h1753, _, _, _, _, _, _, _, _) => {
            h1753
        }
    }), List::cons(f33(match v0.clone() {
        Adt4::C0(_, _, _, _, h1754, _, _, _, _, _, _, _) => {
            h1754
        }
    }), List::cons(f78(f94(v0.clone())), List::cons(f78(match v0 {
        Adt4::C0(_, _, _, _, _, _, h1755, _, _, _, _, _) => {
            h1755
        }
    }), List::<Bytes>::nil())))))))))
}

pub fn f76(v0: u64, v1: Adt4) -> R<Bytes> {
    let v2: Adt13 = f95(match v1.clone() {
        Adt4::C0(_, _, _, _, _, h1756, _, _, _, _, _, _) => {
            h1756
        }
    }, match v1.clone() {
        Adt4::C0(_, _, _, _, _, _, h1757, _, _, _, _, _) => {
            h1757
        }
    }, match v1.clone() {
        Adt4::C0(_, _, _, _, _, _, _, h1758, _, _, _, _) => {
            h1758
        }
    }, match v1.clone() {
        Adt4::C0(_, _, _, _, _, _, _, _, h1759, _, _, _) => {
            h1759
        }
    }, match v1.clone() {
        Adt4::C0(_, _, _, _, _, _, _, _, _, h1760, _, _) => {
            h1760
        }
    }, match v1.clone() {
        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1761, _) => {
            h1761
        }
    })?;
    Ok(f2(List::cons(f33(match v1.clone() {
        Adt4::C0(_, _, _, _, h1762, _, _, _, _, _, _, _) => {
            h1762
        }
    }), List::cons(f70(match v1.clone() {
        Adt4::C0(_, _, _, _, _, _, _, _, _, h1763, _, _) => {
            h1763
        }
    }), List::cons(f78(match v1 {
        Adt4::C0(_, _, _, _, _, _, h1764, _, _, _, _, _) => {
            h1764
        }
    }), List::cons(Bytes::lit(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f]), f96(Fn14::F97(v0), List::cons(f78(f98(v2.clone())), List::<Bytes>::nil()), match v2 {
        Adt13::C0(_, _, _, _, _, _, h1765) => {
            h1765
        }
    })))))))
}

pub fn f77(v0: Adt5) -> Bytes {
    let a1786 = f64(match v0.clone() {
        Adt5::C0(h1766, _, _, _, _, _, _, _) => {
            h1766
        }
    });
    let a1787 = {
        let a1784 = f64(match v0.clone() {
            Adt5::C0(_, h1767, _, _, _, _, _, _) => {
                h1767
            }
        });
        let a1785 = {
            let a1782 = f64(match v0.clone() {
                Adt5::C0(_, _, h1768, _, _, _, _, _) => {
                    h1768
                }
            });
            let a1783 = {
                let a1780 = f64(match v0.clone() {
                    Adt5::C0(_, _, _, h1769, _, _, _, _) => {
                        h1769
                    }
                });
                let a1781 = {
                    let a1778 = f64(match v0.clone() {
                        Adt5::C0(_, _, _, _, h1770, _, _, _) => {
                            h1770
                        }
                    });
                    let a1779 = {
                        let a1776 = f64(match v0.clone() {
                            Adt5::C0(_, _, _, _, _, h1771, _, _) => {
                                h1771
                            }
                        });
                        let a1777 = {
                            let a1774 = f64(match v0.clone() {
                                Adt5::C0(_, _, _, _, _, _, h1772, _) => {
                                    h1772
                                }
                            });
                            let a1775 = f64(match v0 {
                                Adt5::C0(_, _, _, _, _, _, _, h1773) => {
                                    h1773
                                }
                            });
                            append_bytes(a1774, a1775)
                        };
                        append_bytes(a1776, a1777)
                    };
                    append_bytes(a1778, a1779)
                };
                append_bytes(a1780, a1781)
            };
            append_bytes(a1782, a1783)
        };
        append_bytes(a1784, a1785)
    };
    append_bytes(a1786, a1787)
}

pub fn f78(v0: Adt6) -> Bytes {
    let a1832 = f64(match v0.clone() {
        Adt6::C0(h1788, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1788
        }
    });
    let a1833 = {
        let a1830 = f64(match v0.clone() {
            Adt6::C0(_, h1789, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                h1789
            }
        });
        let a1831 = {
            let a1828 = f64(match v0.clone() {
                Adt6::C0(_, _, h1790, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                    h1790
                }
            });
            let a1829 = {
                let a1826 = f64(match v0.clone() {
                    Adt6::C0(_, _, _, h1791, _, _, _, _, _, _, _, _, _, _, _, _) => {
                        h1791
                    }
                });
                let a1827 = {
                    let a1824 = f64(match v0.clone() {
                        Adt6::C0(_, _, _, _, h1792, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1792
                        }
                    });
                    let a1825 = {
                        let a1822 = f64(match v0.clone() {
                            Adt6::C0(_, _, _, _, _, h1793, _, _, _, _, _, _, _, _, _, _) => {
                                h1793
                            }
                        });
                        let a1823 = {
                            let a1820 = f64(match v0.clone() {
                                Adt6::C0(_, _, _, _, _, _, h1794, _, _, _, _, _, _, _, _, _) => {
                                    h1794
                                }
                            });
                            let a1821 = {
                                let a1818 = f64(match v0.clone() {
                                    Adt6::C0(_, _, _, _, _, _, _, h1795, _, _, _, _, _, _, _, _) => {
                                        h1795
                                    }
                                });
                                let a1819 = {
                                    let a1816 = f64(match v0.clone() {
                                        Adt6::C0(_, _, _, _, _, _, _, _, h1796, _, _, _, _, _, _, _) => {
                                            h1796
                                        }
                                    });
                                    let a1817 = {
                                        let a1814 = f64(match v0.clone() {
                                            Adt6::C0(_, _, _, _, _, _, _, _, _, h1797, _, _, _, _, _, _) => {
                                                h1797
                                            }
                                        });
                                        let a1815 = {
                                            let a1812 = f64(match v0.clone() {
                                                Adt6::C0(_, _, _, _, _, _, _, _, _, _, h1798, _, _, _, _, _) => {
                                                    h1798
                                                }
                                            });
                                            let a1813 = {
                                                let a1810 = f64(match v0.clone() {
                                                    Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h1799, _, _, _, _) => {
                                                        h1799
                                                    }
                                                });
                                                let a1811 = {
                                                    let a1808 = f64(match v0.clone() {
                                                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1800, _, _, _) => {
                                                            h1800
                                                        }
                                                    });
                                                    let a1809 = {
                                                        let a1806 = f64(match v0.clone() {
                                                            Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1801, _, _) => {
                                                                h1801
                                                            }
                                                        });
                                                        let a1807 = {
                                                            let a1804 = f64(match v0.clone() {
                                                                Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1802, _) => {
                                                                    h1802
                                                                }
                                                            });
                                                            let a1805 = f64(match v0 {
                                                                Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1803) => {
                                                                    h1803
                                                                }
                                                            });
                                                            append_bytes(a1804, a1805)
                                                        };
                                                        append_bytes(a1806, a1807)
                                                    };
                                                    append_bytes(a1808, a1809)
                                                };
                                                append_bytes(a1810, a1811)
                                            };
                                            append_bytes(a1812, a1813)
                                        };
                                        append_bytes(a1814, a1815)
                                    };
                                    append_bytes(a1816, a1817)
                                };
                                append_bytes(a1818, a1819)
                            };
                            append_bytes(a1820, a1821)
                        };
                        append_bytes(a1822, a1823)
                    };
                    append_bytes(a1824, a1825)
                };
                append_bytes(a1826, a1827)
            };
            append_bytes(a1828, a1829)
        };
        append_bytes(a1830, a1831)
    };
    append_bytes(a1832, a1833)
}

pub fn f79(v0: Fn11, p1837: u64, p1838: (u64, Adt11)) -> R<(u64, Adt11)> {
    let m1836 = (p1837, p1838);
    {
        let mut m1836 = m1836;
        loop {
            let step = {
                let (v1, v2) = m1836;
                let m1834 = v1;
                if m1834 == 0 {
                    Err::<(u64, (u64, Adt11)), R<(u64, Adt11)>>(Ok(v2))
                } else {
                    let v3 = m1834 - 1;
                    Ok::<(u64, (u64, Adt11)), R<(u64, Adt11)>>((v3, {
                        let c1835: Fn11 = v0.clone();
                        c1835.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m1836 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f80(v0: Bytes, v1: u64, v2: u64, v3: (u64, Adt11)) -> R<(u64, Adt11)> {
    Ok(({
        let a1840 = {
            let (h1839, _) = v3.clone();
            h1839
        };
        let a1841 = 64u64;
        nat_add(a1840, a1841)?
    }, f99({
        let (_, h1842) = v3;
        h1842
    }, f100(v0, v1, v2, {
        let (h1843, _) = v3;
        h1843
    })?)?))
}

pub fn f81() -> Adt11 {
    Adt11::C0(1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, 1359893119u32, 2600822924u32, 528734635u32, 1541459225u32)
}

pub fn f82(v0: u32, v1: u32) -> u32 {
    let m1846 = {
        let a1844 = v0;
        let a1845 = v1;
        fixed_u32::shift_right(a1844, a1845)
    };
    match m1846 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f83(v0: Bytes, v1: u64, v2: u64) -> R<u32> {
    Ok(f101(f102(v0.clone(), v1, v2), f102(v0.clone(), v1, {
        let a1847 = v2;
        let a1848 = 1u64;
        nat_add(a1847, a1848)?
    }), f102(v0.clone(), v1, {
        let a1849 = v2;
        let a1850 = 2u64;
        nat_add(a1849, a1850)?
    }), f102(v0, v1, {
        let a1851 = v2;
        let a1852 = 3u64;
        nat_add(a1851, a1852)?
    })))
}

pub fn f84(v0: Fn12, p1856: Adt6, p1857: List<Adt3>) -> Adt6 {
    let m1855 = (p1856, p1857);
    {
        let mut m1855 = m1855;
        loop {
            let step = {
                let (v1, v2) = m1855;
                let m1853 = v2;
                match m1853.uncons() {
                    None => {
                        Err::<(Adt6, List<Adt3>), Adt6>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt6, List<Adt3>), Adt6>(({
                            let c1854: Fn12 = v0.clone();
                            c1854.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m1855 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f85(v0: u64, v1: Adt6, v2: Adt3) -> Adt6 {
    let m1858 = v2;
    match m1858 {
        Adt3::C0(v3) => {
            let a1860 = match v3.clone() {
                Adt4::C0(h1859, _, _, _, _, _, _, _, _, _, _, _) => {
                    h1859
                }
            };
            let a1861 = 2u64;
            if nat_eq(a1860, a1861) {
                let a1863 = match v3.clone() {
                    Adt4::C0(_, _, _, h1862, _, _, _, _, _, _, _, _) => {
                        h1862
                    }
                };
                let a1864 = v0;
                if nat_eq(a1863, a1864) {
                    match v3 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1865) => {
                            h1865
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

pub fn f86() -> u32 {
    1u32
}

pub fn f87(v0: Adt5, v1: Adt6, v2: u32, v3: u32, v4: u32, v5: u32) -> R<Adt6> {
    Ok(f98(f95(v0, v1, v2, v3, v4, v5)?))
}

pub fn f88() -> u32 {
    4u32
}

pub fn f89() -> u32 {
    8u32
}

pub fn f90(v0: Adt5, v1: u32, v2: Adt12) -> R<(List<Adt7>, List<Adt3>)> {
    let v3: Adt12 = {
        let (h1866, _) = f103(Fn15::F104(v1, v0), 64u64, v2)?;
        h1866
    };
    let v4: List<Adt7> = List::cons(match v3.clone() {
        Adt12::C0(h1867, _, _, _) => {
            h1867
        }
    }, match v3.clone() {
        Adt12::C0(_, _, h1868, _) => {
            h1868
        }
    });
    Ok((v4.clone(), List::cons(Adt3::C1(match v3.clone() {
        Adt12::C0(h1869, _, _, _) => {
            h1869
        }
    }, {
        let a1870 = v4;
        length_list(a1870)
    }), match v3 {
        Adt12::C0(_, _, _, h1871) => {
            h1871
        }
    })))
}

pub fn f91(v0: u64) -> Bytes {
    f64(f71(v0, 32u64))
}

pub fn f92(v0: Fn13, p1875: u64, p1876: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let m1874 = (p1875, p1876);
    {
        let mut m1874 = m1874;
        loop {
            let step = {
                let (v1, v2) = m1874;
                let m1872 = v1;
                if m1872 == 0 {
                    Err::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>(v2)
                } else {
                    let v3 = m1872 - 1;
                    Ok::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>((v3, {
                        let c1873: Fn13 = v0.clone();
                        c1873.apply(v2)
                    }))
                }
            };
            match step {
                Ok(next) => { m1874 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f93(v0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let v1: u64 = {
        let (h1877, _) = v0;
        h1877
    };
    let v2: u32 = {
        let (h1878, _) = {
            let (_, h1879) = v0;
            h1879
        };
        h1878
    };
    let v3: u32 = {
        let (_, h1880) = {
            let (_, h1881) = v0;
            h1881
        };
        h1880
    };
    ({
        let a1882 = v1;
        let a1883 = 2u64;
        let a1884 = 0u64;
        nat_quot(a1882, a1883, a1884)
    }, (f105(v2, 1u32), {
        let a1888 = {
            let a1885 = v1;
            let a1886 = 2u64;
            let a1887 = 0u64;
            nat_rem(a1885, a1886, a1887)
        };
        let a1889 = 1u64;
        if nat_eq(a1888, a1889) {
            let a1890 = v3;
            let a1891 = v2;
            fixed_u32::bit_or(a1890, a1891)
        } else {
            v3
        }
    }))
}

pub fn f94(v0: Adt4) -> Adt6 {
    f106(match v0.clone() {
        Adt4::C0(_, _, _, _, _, h1892, _, _, _, _, _, _) => {
            h1892
        }
    }, match v0.clone() {
        Adt4::C0(_, _, _, _, _, _, _, h1893, _, _, _, _) => {
            h1893
        }
    }, match v0.clone() {
        Adt4::C0(_, _, _, _, _, _, _, _, h1894, _, _, _) => {
            h1894
        }
    }, match v0.clone() {
        Adt4::C0(_, _, _, _, _, _, _, _, _, h1895, _, _) => {
            h1895
        }
    }, match v0 {
        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1896, _) => {
            h1896
        }
    })
}

pub fn f95(v0: Adt5, v1: Adt6, v2: u32, v3: u32, v4: u32, v5: u32) -> R<Adt13> {
    let v6: (u64, Adt16) = f107(Fn16::F108(v0.clone(), v1.clone()), 800u64, (0u64, Adt16::C0(f106(v0.clone(), v2, v3, v4, v5), Adt6::C0(0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32), List::<Adt14>::nil())))?;
    Ok(Adt13::C0(v0, v1, v2, v3, v4, v5, {
        let m1899 = {
            let (_, h1898) = v6;
            h1898
        };
        match m1899 {
            Adt16::C0(_, _, h1897) => {
                h1897
            }
        }
    }))
}

pub fn f96(v0: Fn14, p1903: List<Bytes>, p1904: List<Adt14>) -> List<Bytes> {
    let m1902 = (p1903, p1904);
    {
        let mut m1902 = m1902;
        loop {
            let step = {
                let (v1, v2) = m1902;
                let m1900 = v2;
                match m1900.uncons() {
                    None => {
                        Err::<(List<Bytes>, List<Adt14>), List<Bytes>>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(List<Bytes>, List<Adt14>), List<Bytes>>(({
                            let c1901: Fn14 = v0.clone();
                            c1901.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m1902 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f97(v0: u64, v1: List<Bytes>, v2: Adt14) -> List<Bytes> {
    List::cons(if f109(v2.clone()) {
        let a1906 = f110(match v2.clone() {
            Adt14::C0(h1905, _, _, _, _, _, _, _, _) => {
                h1905
            }
        });
        let a1907 = f111(v0, v2);
        append_bytes(a1906, a1907)
    } else {
        f111(v0, v2)
    }, v1)
}

pub fn f98(v0: Adt13) -> Adt6 {
    f112(Fn17::F113, Adt6::C0(0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32), match v0 {
        Adt13::C0(_, _, _, _, _, _, h1908) => {
            h1908
        }
    })
}

pub fn f99(v0: Adt11, v1: Adt17) -> R<Adt11> {
    let v2: Adt18 = f114(Fn18::F115, 64u64, Adt18::C0(match v0.clone() {
        Adt11::C0(h1909, _, _, _, _, _, _, _) => {
            h1909
        }
    }, match v0.clone() {
        Adt11::C0(_, h1910, _, _, _, _, _, _) => {
            h1910
        }
    }, match v0.clone() {
        Adt11::C0(_, _, h1911, _, _, _, _, _) => {
            h1911
        }
    }, match v0.clone() {
        Adt11::C0(_, _, _, h1912, _, _, _, _) => {
            h1912
        }
    }, match v0.clone() {
        Adt11::C0(_, _, _, _, h1913, _, _, _) => {
            h1913
        }
    }, match v0.clone() {
        Adt11::C0(_, _, _, _, _, h1914, _, _) => {
            h1914
        }
    }, match v0.clone() {
        Adt11::C0(_, _, _, _, _, _, h1915, _) => {
            h1915
        }
    }, match v0.clone() {
        Adt11::C0(_, _, _, _, _, _, _, h1916) => {
            h1916
        }
    }, v1, 0u64))?;
    Ok(Adt11::C0(f116(match v0.clone() {
        Adt11::C0(h1917, _, _, _, _, _, _, _) => {
            h1917
        }
    }, match v2.clone() {
        Adt18::C0(h1918, _, _, _, _, _, _, _, _, _) => {
            h1918
        }
    }), f116(match v0.clone() {
        Adt11::C0(_, h1919, _, _, _, _, _, _) => {
            h1919
        }
    }, match v2.clone() {
        Adt18::C0(_, h1920, _, _, _, _, _, _, _, _) => {
            h1920
        }
    }), f116(match v0.clone() {
        Adt11::C0(_, _, h1921, _, _, _, _, _) => {
            h1921
        }
    }, match v2.clone() {
        Adt18::C0(_, _, h1922, _, _, _, _, _, _, _) => {
            h1922
        }
    }), f116(match v0.clone() {
        Adt11::C0(_, _, _, h1923, _, _, _, _) => {
            h1923
        }
    }, match v2.clone() {
        Adt18::C0(_, _, _, h1924, _, _, _, _, _, _) => {
            h1924
        }
    }), f116(match v0.clone() {
        Adt11::C0(_, _, _, _, h1925, _, _, _) => {
            h1925
        }
    }, match v2.clone() {
        Adt18::C0(_, _, _, _, h1926, _, _, _, _, _) => {
            h1926
        }
    }), f116(match v0.clone() {
        Adt11::C0(_, _, _, _, _, h1927, _, _) => {
            h1927
        }
    }, match v2.clone() {
        Adt18::C0(_, _, _, _, _, h1928, _, _, _, _) => {
            h1928
        }
    }), f116(match v0.clone() {
        Adt11::C0(_, _, _, _, _, _, h1929, _) => {
            h1929
        }
    }, match v2.clone() {
        Adt18::C0(_, _, _, _, _, _, h1930, _, _, _) => {
            h1930
        }
    }), f116(match v0 {
        Adt11::C0(_, _, _, _, _, _, _, h1931) => {
            h1931
        }
    }, match v2 {
        Adt18::C0(_, _, _, _, _, _, _, h1932, _, _) => {
            h1932
        }
    })))
}

pub fn f100(v0: Bytes, v1: u64, v2: u64, v3: u64) -> R<Adt17> {
    Ok(Adt17::C0(f117(v0.clone(), v1, v2, v3)?, f117(v0.clone(), v1, v2, {
        let a1933 = v3;
        let a1934 = 4u64;
        nat_add(a1933, a1934)?
    })?, f117(v0.clone(), v1, v2, {
        let a1935 = v3;
        let a1936 = 8u64;
        nat_add(a1935, a1936)?
    })?, f117(v0.clone(), v1, v2, {
        let a1937 = v3;
        let a1938 = 12u64;
        nat_add(a1937, a1938)?
    })?, f117(v0.clone(), v1, v2, {
        let a1939 = v3;
        let a1940 = 16u64;
        nat_add(a1939, a1940)?
    })?, f117(v0.clone(), v1, v2, {
        let a1941 = v3;
        let a1942 = 20u64;
        nat_add(a1941, a1942)?
    })?, f117(v0.clone(), v1, v2, {
        let a1943 = v3;
        let a1944 = 24u64;
        nat_add(a1943, a1944)?
    })?, f117(v0.clone(), v1, v2, {
        let a1945 = v3;
        let a1946 = 28u64;
        nat_add(a1945, a1946)?
    })?, f117(v0.clone(), v1, v2, {
        let a1947 = v3;
        let a1948 = 32u64;
        nat_add(a1947, a1948)?
    })?, f117(v0.clone(), v1, v2, {
        let a1949 = v3;
        let a1950 = 36u64;
        nat_add(a1949, a1950)?
    })?, f117(v0.clone(), v1, v2, {
        let a1951 = v3;
        let a1952 = 40u64;
        nat_add(a1951, a1952)?
    })?, f117(v0.clone(), v1, v2, {
        let a1953 = v3;
        let a1954 = 44u64;
        nat_add(a1953, a1954)?
    })?, f117(v0.clone(), v1, v2, {
        let a1955 = v3;
        let a1956 = 48u64;
        nat_add(a1955, a1956)?
    })?, f117(v0.clone(), v1, v2, {
        let a1957 = v3;
        let a1958 = 52u64;
        nat_add(a1957, a1958)?
    })?, f117(v0.clone(), v1, v2, {
        let a1959 = v3;
        let a1960 = 56u64;
        nat_add(a1959, a1960)?
    })?, f117(v0, v1, v2, {
        let a1961 = v3;
        let a1962 = 60u64;
        nat_add(a1961, a1962)?
    })?))
}

pub fn f101(v0: u32, v1: u32, v2: u32, v3: u32) -> u32 {
    let a1967 = v0;
    let a1968 = {
        let a1965 = f105(v1, 8u32);
        let a1966 = {
            let a1963 = f105(v2, 16u32);
            let a1964 = f105(v3, 24u32);
            fixed_u32::bit_or(a1963, a1964)
        };
        fixed_u32::bit_or(a1965, a1966)
    };
    fixed_u32::bit_or(a1967, a1968)
}

pub fn f102(v0: Bytes, v1: u64, v2: u64) -> u32 {
    let a1969 = v2;
    let a1970 = v1;
    if nat_lt(a1969, a1970) {
        let m1973 = {
            let a1971 = v0;
            let a1972 = v2;
            index_bytes(a1971, a1972)
        };
        match m1973 {
            None => {
                0u32
            }
            Some(v3) => {
                let m1975 = {
                    let a1974 = v3;
                    fixed_u32::convert(i128::from(a1974))
                };
                match m1975 {
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

pub fn f103(v0: Fn15, p1980: u64, p1981: Adt12) -> R<(Adt12, bool)> {
    let m1979 = (p1980, p1981);
    {
        let mut m1979 = m1979;
        loop {
            let step = {
                let (v1, v2) = m1979;
                let m1976 = v1;
                if m1976 == 0 {
                    Err::<(u64, Adt12), R<(Adt12, bool)>>(Ok((v2, false)))
                } else {
                    let v3 = m1976 - 1;
                    let m1978 = {
                        let c1977: Fn15 = v0.clone();
                        c1977.apply(v2.clone())?
                    };
                    match m1978 {
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
                Ok(next) => { m1979 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f104(v0: u32, v1: Adt5, v2: Adt12) -> R<Option<Adt12>> {
    f118(v1, v0, v2)
}

pub fn f105(v0: u32, v1: u32) -> u32 {
    let m1984 = {
        let a1982 = v0;
        let a1983 = v1;
        fixed_u32::shift_left(a1982, a1983)
    };
    match m1984 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f106(v0: Adt5, v1: u32, v2: u32, v3: u32, v4: u32) -> Adt6 {
    Adt6::C0(match v0.clone() {
        Adt5::C0(h1985, _, _, _, _, _, _, _) => {
            h1985
        }
    }, match v0.clone() {
        Adt5::C0(_, h1986, _, _, _, _, _, _) => {
            h1986
        }
    }, match v0.clone() {
        Adt5::C0(_, _, h1987, _, _, _, _, _) => {
            h1987
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, h1988, _, _, _, _) => {
            h1988
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, h1989, _, _, _) => {
            h1989
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, h1990, _, _) => {
            h1990
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, h1991, _) => {
            h1991
        }
    }, match v0 {
        Adt5::C0(_, _, _, _, _, _, _, h1992) => {
            h1992
        }
    }, 1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, v1, v2, v3, v4)
}

pub fn f107(v0: Fn16, p1996: u64, p1997: (u64, Adt16)) -> R<(u64, Adt16)> {
    let m1995 = (p1996, p1997);
    {
        let mut m1995 = m1995;
        loop {
            let step = {
                let (v1, v2) = m1995;
                let m1993 = v1;
                if m1993 == 0 {
                    Err::<(u64, (u64, Adt16)), R<(u64, Adt16)>>(Ok(v2))
                } else {
                    let v3 = m1993 - 1;
                    Ok::<(u64, (u64, Adt16)), R<(u64, Adt16)>>((v3, {
                        let c1994: Fn16 = v0.clone();
                        c1994.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m1995 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f108(v0: Adt5, v1: Adt6, v2: (u64, Adt16)) -> R<(u64, Adt16)> {
    Ok(({
        let a1999 = {
            let (h1998, _) = v2.clone();
            h1998
        };
        let a2000 = 1u64;
        nat_add(a1999, a2000)?
    }, f119(v0, v1, {
        let (_, h2001) = v2;
        h2001
    }, {
        let (h2002, _) = v2;
        h2002
    })?))
}

pub fn f109(v0: Adt14) -> bool {
    let a2004 = match v0.clone() {
        Adt14::C0(h2003, _, _, _, _, _, _, _, _) => {
            h2003
        }
    };
    let a2005 = 7u64;
    if nat_lt(a2004, a2005) {
        let a2007 = match v0.clone() {
            Adt14::C0(_, h2006, _, _, _, _, _, _, _) => {
                h2006
            }
        };
        let a2008 = 0u64;
        if nat_eq(a2007, a2008) {
            let a2010 = match v0 {
                Adt14::C0(_, _, h2009, _, _, _, _, _, _) => {
                    h2009
                }
            };
            let a2011 = 0u64;
            nat_eq(a2010, a2011)
        } else {
            false
        }
    } else {
        false
    }
}

pub fn f110(v0: u64) -> Bytes {
    let a2012 = v0;
    let a2013 = 3u64;
    if nat_lt(a2012, a2013) {
        let a2014 = v0;
        let a2015 = 1u64;
        if nat_lt(a2014, a2015) {
            Bytes::lit(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f])
        } else {
            let a2016 = v0;
            let a2017 = 2u64;
            if nat_lt(a2016, a2017) {
                Bytes::lit(&[0x02, 0x06, 0x03, 0x0a, 0x07, 0x00, 0x04, 0x0d, 0x01, 0x0b, 0x0c, 0x05, 0x09, 0x0e, 0x0f, 0x08])
            } else {
                Bytes::lit(&[0x03, 0x04, 0x0a, 0x0c, 0x0d, 0x02, 0x07, 0x0e, 0x06, 0x05, 0x09, 0x00, 0x0b, 0x0f, 0x08, 0x01])
            }
        }
    } else {
        let a2018 = v0;
        let a2019 = 5u64;
        if nat_lt(a2018, a2019) {
            let a2020 = v0;
            let a2021 = 4u64;
            if nat_lt(a2020, a2021) {
                Bytes::lit(&[0x0a, 0x07, 0x0c, 0x09, 0x0e, 0x03, 0x0d, 0x0f, 0x04, 0x00, 0x0b, 0x02, 0x05, 0x08, 0x01, 0x06])
            } else {
                Bytes::lit(&[0x0c, 0x0d, 0x09, 0x0b, 0x0f, 0x0a, 0x0e, 0x08, 0x07, 0x02, 0x05, 0x03, 0x00, 0x01, 0x06, 0x04])
            }
        } else {
            let a2022 = v0;
            let a2023 = 6u64;
            if nat_lt(a2022, a2023) {
                Bytes::lit(&[0x09, 0x0e, 0x0b, 0x05, 0x08, 0x0c, 0x0f, 0x01, 0x0d, 0x03, 0x00, 0x0a, 0x02, 0x06, 0x04, 0x07])
            } else {
                Bytes::lit(&[0x0b, 0x0f, 0x05, 0x00, 0x01, 0x09, 0x08, 0x06, 0x0e, 0x0a, 0x02, 0x0c, 0x03, 0x04, 0x07, 0x0d])
            }
        }
    }
}

pub fn f111(v0: u64, v1: Adt14) -> Bytes {
    f2(List::cons(f37(match v1.clone() {
        Adt14::C0(h2024, _, _, _, _, _, _, _, _) => {
            h2024
        }
    }), List::cons(f37(match v1.clone() {
        Adt14::C0(_, h2025, _, _, _, _, _, _, _) => {
            h2025
        }
    }), List::cons(f37(match v1.clone() {
        Adt14::C0(_, _, h2026, _, _, _, _, _, _) => {
            h2026
        }
    }), List::cons(f37(f120(match v1.clone() {
        Adt14::C0(_, _, _, h2027, _, _, _, _, _) => {
            h2027
        }
    })), List::cons(f37(match v1.clone() {
        Adt14::C0(_, _, _, _, h2028, _, _, _, _) => {
            h2028
        }
    }), List::cons(f37(match v1.clone() {
        Adt14::C0(_, _, _, _, _, h2029, _, _, _) => {
            h2029
        }
    }), List::cons(f64(match v1.clone() {
        Adt14::C0(_, _, _, _, _, _, h2030, _, _) => {
            h2030
        }
    }), List::cons(f64(match v1.clone() {
        Adt14::C0(_, _, _, _, _, _, _, h2031, _) => {
            h2031
        }
    }), List::cons(f64(match v1.clone() {
        Adt14::C0(_, _, _, _, _, _, _, _, h2032) => {
            h2032
        }
    }), List::cons(f121(v0, v1), List::<Bytes>::nil())))))))))))
}

pub fn f112(v0: Fn17, p2036: Adt6, p2037: List<Adt14>) -> Adt6 {
    let m2035 = (p2036, p2037);
    {
        let mut m2035 = m2035;
        loop {
            let step = {
                let (v1, v2) = m2035;
                let m2033 = v2;
                match m2033.uncons() {
                    None => {
                        Err::<(Adt6, List<Adt14>), Adt6>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt6, List<Adt14>), Adt6>(({
                            let c2034: Fn17 = v0.clone();
                            c2034.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m2035 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f113(v0: Adt6, v1: Adt14) -> Adt6 {
    let a2039 = match v1.clone() {
        Adt14::C0(h2038, _, _, _, _, _, _, _, _) => {
            h2038
        }
    };
    let a2040 = 7u64;
    if nat_eq(a2039, a2040) {
        f122(v0, match v1.clone() {
            Adt14::C0(_, _, _, _, h2041, _, _, _, _) => {
                h2041
            }
        }, match v1 {
            Adt14::C0(_, _, _, _, _, _, _, _, h2042) => {
                h2042
            }
        })
    } else {
        v0
    }
}

pub fn f114(v0: Fn18, p2046: u64, p2047: Adt18) -> R<Adt18> {
    let m2045 = (p2046, p2047);
    {
        let mut m2045 = m2045;
        loop {
            let step = {
                let (v1, v2) = m2045;
                let m2043 = v1;
                if m2043 == 0 {
                    Err::<(u64, Adt18), R<Adt18>>(Ok(v2))
                } else {
                    let v3 = m2043 - 1;
                    Ok::<(u64, Adt18), R<Adt18>>((v3, {
                        let c2044: Fn18 = v0.clone();
                        c2044.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m2045 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f115(v0: Adt18) -> R<Adt18> {
    f123(v0)
}

pub fn f116(v0: u32, v1: u32) -> u32 {
    let m2050 = {
        let a2048 = f124(v0);
        let a2049 = f124(v1);
        fixed_u64::checked_add(a2048, a2049)
    };
    match m2050 {
        None => {
            0u32
        }
        Some(v2) => {
            f125(v2)
        }
    }
}

pub fn f117(v0: Bytes, v1: u64, v2: u64, v3: u64) -> R<u32> {
    let a2053 = {
        let a2051 = v3;
        let a2052 = 8u64;
        nat_add(a2051, a2052)?
    };
    let a2054 = v2;
    if nat_eq(a2053, a2054) {
        Ok(f126({
            let a2057 = {
                let a2055 = v1;
                let a2056 = 8u64;
                nat_mul(a2055, a2056)?
            };
            let a2058 = 4294967296u64;
            let a2059 = 0u64;
            nat_quot(a2057, a2058, a2059)
        }))
    } else {
        let a2062 = {
            let a2060 = v3;
            let a2061 = 4u64;
            nat_add(a2060, a2061)?
        };
        let a2063 = v2;
        if nat_eq(a2062, a2063) {
            Ok(f126({
                let a2066 = {
                    let a2064 = v1;
                    let a2065 = 8u64;
                    nat_mul(a2064, a2065)?
                };
                let a2067 = 4294967296u64;
                let a2068 = 0u64;
                nat_rem(a2066, a2067, a2068)
            }))
        } else {
            Ok(f127(f128(v0.clone(), v1, v3), f128(v0.clone(), v1, {
                let a2069 = v3;
                let a2070 = 1u64;
                nat_add(a2069, a2070)?
            }), f128(v0.clone(), v1, {
                let a2071 = v3;
                let a2072 = 2u64;
                nat_add(a2071, a2072)?
            }), f128(v0, v1, {
                let a2073 = v3;
                let a2074 = 3u64;
                nat_add(a2073, a2074)?
            })))
        }
    }
}

pub fn f118(v0: Adt5, v1: u32, v2: Adt12) -> R<Option<Adt12>> {
    let a2079 = {
        let a2076 = match v2.clone() {
            Adt12::C0(_, h2075, _, _) => {
                h2075
            }
        };
        let a2077 = 2u64;
        let a2078 = 0u64;
        nat_rem(a2076, a2077, a2078)
    };
    let a2080 = 0u64;
    if nat_eq(a2079, a2080) {
        let m2082 = match v2.clone() {
            Adt12::C0(_, _, h2081, _) => {
                h2081
            }
        };
        match m2082.uncons() {
            None => {
                Ok(None::<Adt12>)
            }
            Some((v3, v4)) => {
                let v5: Adt4 = f57(f58(v3.clone(), match v2.clone() {
                    Adt12::C0(h2083, _, _, _) => {
                        h2083
                    }
                }, v0, v1)?)?;
                Ok(Some(Adt12::C0(Adt7::C0(f46(match v5.clone() {
                    Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h2084) => {
                        h2084
                    }
                }), match v3.clone() {
                    Adt7::C0(_, h2085, _) => {
                        h2085
                    }
                }, {
                    let a2090 = match v3.clone() {
                        Adt7::C0(_, _, h2086) => {
                            h2086
                        }
                    };
                    let a2091 = {
                        let m2089 = match v2.clone() {
                            Adt12::C0(h2088, _, _, _) => {
                                h2088
                            }
                        };
                        match m2089 {
                            Adt7::C0(_, _, h2087) => {
                                h2087
                            }
                        }
                    };
                    nat_add(a2090, a2091)?
                }), {
                    let a2093 = match v2.clone() {
                        Adt12::C0(_, h2092, _, _) => {
                            h2092
                        }
                    };
                    let a2094 = 2u64;
                    let a2095 = 0u64;
                    nat_quot(a2093, a2094, a2095)
                }, v4, List::cons(Adt3::C0(v5), List::cons(Adt3::C2(v3, {
                    let a2097 = match v2.clone() {
                        Adt12::C0(_, _, h2096, _) => {
                            h2096
                        }
                    };
                    length_list(a2097)
                }), match v2 {
                    Adt12::C0(_, _, _, h2098) => {
                        h2098
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
        let a2099 = v3;
        let a2100 = 784u64;
        if nat_lt(a2099, a2100) {
            let a2101 = v3;
            let a2102 = 112u64;
            let a2103 = 0u64;
            nat_quot(a2101, a2102, a2103)
        } else {
            7u64
        }
    };
    let v5: Adt19 = f129({
        let a2104 = v3;
        let a2105 = 784u64;
        if nat_lt(a2104, a2105) {
            let a2106 = v3;
            let a2107 = 112u64;
            let a2108 = 0u64;
            nat_rem(a2106, a2107, a2108)
        } else {
            let a2109 = v3;
            let a2110 = 672u64;
            nat_sub(a2109, a2110)
        }
    });
    let v6: Adt6 = match v2.clone() {
        Adt16::C0(h2111, _, _) => {
            h2111
        }
    };
    let v7: u32 = f130(v6.clone(), match v5.clone() {
        Adt19::C0(_, _, _, h2112, _) => {
            h2112
        }
    });
    let v8: u32 = {
        let m2114 = match v5.clone() {
            Adt19::C0(_, _, h2113, _, _) => {
                h2113
            }
        };
        match m2114 {
            Adt15::C0 => {
                f130(v6.clone(), match v5.clone() {
                    Adt19::C0(_, _, _, _, h2115) => {
                        h2115
                    }
                })
            }
            Adt15::C1 => {
                f130(v1, f131(v4, match v5.clone() {
                    Adt19::C0(_, _, _, _, h2116) => {
                        h2116
                    }
                })?)
            }
            Adt15::C2 => {
                f130(v6.clone(), match v5.clone() {
                    Adt19::C0(_, _, _, _, h2117) => {
                        h2117
                    }
                })
            }
            Adt15::C3 => {
                f132(match v5.clone() {
                    Adt19::C0(_, _, _, _, h2118) => {
                        h2118
                    }
                })
            }
            Adt15::C4 => {
                f133(v0, match v5.clone() {
                    Adt19::C0(_, _, _, _, h2119) => {
                        h2119
                    }
                })
            }
        }
    };
    let v9: u32 = f134(match v5.clone() {
        Adt19::C0(_, _, h2120, _, _) => {
            h2120
        }
    }, v7, v8);
    Ok(Adt16::C0({
        let a2121 = v4;
        let a2122 = 7u64;
        if nat_eq(a2121, a2122) {
            v6
        } else {
            f122(v6, match v5.clone() {
                Adt19::C0(_, _, _, h2123, _) => {
                    h2123
                }
            }, v9)
        }
    }, {
        let a2124 = v4;
        let a2125 = 7u64;
        if nat_eq(a2124, a2125) {
            f122(match v2.clone() {
                Adt16::C0(_, h2126, _) => {
                    h2126
                }
            }, match v5.clone() {
                Adt19::C0(_, _, _, h2127, _) => {
                    h2127
                }
            }, v9)
        } else {
            match v2.clone() {
                Adt16::C0(_, h2128, _) => {
                    h2128
                }
            }
        }
    }, List::cons(Adt14::C0(v4, match v5.clone() {
        Adt19::C0(h2129, _, _, _, _) => {
            h2129
        }
    }, match v5.clone() {
        Adt19::C0(_, h2130, _, _, _) => {
            h2130
        }
    }, match v5.clone() {
        Adt19::C0(_, _, h2131, _, _) => {
            h2131
        }
    }, match v5.clone() {
        Adt19::C0(_, _, _, h2132, _) => {
            h2132
        }
    }, match v5 {
        Adt19::C0(_, _, _, _, h2133) => {
            h2133
        }
    }, v7, v8, v9), match v2 {
        Adt16::C0(_, _, h2134) => {
            h2134
        }
    })))
}

pub fn f120(v0: Adt15) -> u64 {
    let m2135 = v0;
    match m2135 {
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
    let a2136 = v0;
    let a2137 = 2u64;
    if nat_lt(a2136, a2137) {
        Bytes::lit(&[])
    } else {
        let m2139 = match v1.clone() {
            Adt14::C0(_, _, _, h2138, _, _, _, _, _) => {
                h2138
            }
        };
        match m2139 {
            Adt15::C0 => {
                f135(match v1.clone() {
                    Adt14::C0(_, _, _, _, _, _, h2140, _, _) => {
                        h2140
                    }
                }, match v1 {
                    Adt14::C0(_, _, _, _, _, _, _, h2141, _) => {
                        h2141
                    }
                })
            }
            Adt15::C1 => {
                f135(match v1.clone() {
                    Adt14::C0(_, _, _, _, _, _, h2142, _, _) => {
                        h2142
                    }
                }, match v1 {
                    Adt14::C0(_, _, _, _, _, _, _, h2143, _) => {
                        h2143
                    }
                })
            }
            Adt15::C2 => {
                f136(match v1.clone() {
                    Adt14::C0(_, _, _, _, _, _, h2144, _, _) => {
                        h2144
                    }
                }, match v1 {
                    Adt14::C0(_, _, _, _, _, _, _, h2145, _) => {
                        h2145
                    }
                })
            }
            Adt15::C3 => {
                Bytes::lit(&[])
            }
            Adt15::C4 => {
                f136(match v1.clone() {
                    Adt14::C0(_, _, _, _, _, _, h2146, _, _) => {
                        h2146
                    }
                }, match v1 {
                    Adt14::C0(_, _, _, _, _, _, _, h2147, _) => {
                        h2147
                    }
                })
            }
        }
    }
}

pub fn f122(v0: Adt6, v1: u64, v2: u32) -> Adt6 {
    let a2148 = v1;
    let a2149 = 8u64;
    if nat_lt(a2148, a2149) {
        let a2150 = v1;
        let a2151 = 4u64;
        if nat_lt(a2150, a2151) {
            let a2152 = v1;
            let a2153 = 2u64;
            if nat_lt(a2152, a2153) {
                let a2154 = v1;
                let a2155 = 1u64;
                if nat_lt(a2154, a2155) {
                    Adt6::C0(v2, match v0.clone() {
                        Adt6::C0(_, h2156, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2156
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2157, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2157
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2158, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2158
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2159, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2159
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2160, _, _, _, _, _, _, _, _, _, _) => {
                            h2160
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2161, _, _, _, _, _, _, _, _, _) => {
                            h2161
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2162, _, _, _, _, _, _, _, _) => {
                            h2162
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2163, _, _, _, _, _, _, _) => {
                            h2163
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2164, _, _, _, _, _, _) => {
                            h2164
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2165, _, _, _, _, _) => {
                            h2165
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2166, _, _, _, _) => {
                            h2166
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2167, _, _, _) => {
                            h2167
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2168, _, _) => {
                            h2168
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2169, _) => {
                            h2169
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2170) => {
                            h2170
                        }
                    })
                } else {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2171, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2171
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, h2172, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2172
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2173, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2173
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2174, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2174
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2175, _, _, _, _, _, _, _, _, _, _) => {
                            h2175
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2176, _, _, _, _, _, _, _, _, _) => {
                            h2176
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2177, _, _, _, _, _, _, _, _) => {
                            h2177
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2178, _, _, _, _, _, _, _) => {
                            h2178
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2179, _, _, _, _, _, _) => {
                            h2179
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2180, _, _, _, _, _) => {
                            h2180
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2181, _, _, _, _) => {
                            h2181
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2182, _, _, _) => {
                            h2182
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2183, _, _) => {
                            h2183
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2184, _) => {
                            h2184
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2185) => {
                            h2185
                        }
                    })
                }
            } else {
                let a2186 = v1;
                let a2187 = 3u64;
                if nat_lt(a2186, a2187) {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2188, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2188
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2189, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2189
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, h2190, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2190
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2191, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2191
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2192, _, _, _, _, _, _, _, _, _, _) => {
                            h2192
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2193, _, _, _, _, _, _, _, _, _) => {
                            h2193
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2194, _, _, _, _, _, _, _, _) => {
                            h2194
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2195, _, _, _, _, _, _, _) => {
                            h2195
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2196, _, _, _, _, _, _) => {
                            h2196
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2197, _, _, _, _, _) => {
                            h2197
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2198, _, _, _, _) => {
                            h2198
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2199, _, _, _) => {
                            h2199
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2200, _, _) => {
                            h2200
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2201, _) => {
                            h2201
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2202) => {
                            h2202
                        }
                    })
                } else {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2203, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2203
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2204, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2204
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2205, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2205
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2206, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2206
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2207, _, _, _, _, _, _, _, _, _, _) => {
                            h2207
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2208, _, _, _, _, _, _, _, _, _) => {
                            h2208
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2209, _, _, _, _, _, _, _, _) => {
                            h2209
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2210, _, _, _, _, _, _, _) => {
                            h2210
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2211, _, _, _, _, _, _) => {
                            h2211
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2212, _, _, _, _, _) => {
                            h2212
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2213, _, _, _, _) => {
                            h2213
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2214, _, _, _) => {
                            h2214
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2215, _, _) => {
                            h2215
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2216, _) => {
                            h2216
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2217) => {
                            h2217
                        }
                    })
                }
            }
        } else {
            let a2218 = v1;
            let a2219 = 6u64;
            if nat_lt(a2218, a2219) {
                let a2220 = v1;
                let a2221 = 5u64;
                if nat_lt(a2220, a2221) {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2222, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2222
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2223, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2223
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2224, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2224
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2225, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2225
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2226, _, _, _, _, _, _, _, _, _, _) => {
                            h2226
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2227, _, _, _, _, _, _, _, _, _) => {
                            h2227
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2228, _, _, _, _, _, _, _, _) => {
                            h2228
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2229, _, _, _, _, _, _, _) => {
                            h2229
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2230, _, _, _, _, _, _) => {
                            h2230
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2231, _, _, _, _, _) => {
                            h2231
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2232, _, _, _, _) => {
                            h2232
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2233, _, _, _) => {
                            h2233
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2234, _, _) => {
                            h2234
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2235, _) => {
                            h2235
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2236) => {
                            h2236
                        }
                    })
                } else {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2237, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2237
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2238, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2238
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2239, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2239
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2240, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2240
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2241, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2241
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2242, _, _, _, _, _, _, _, _, _) => {
                            h2242
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2243, _, _, _, _, _, _, _, _) => {
                            h2243
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2244, _, _, _, _, _, _, _) => {
                            h2244
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2245, _, _, _, _, _, _) => {
                            h2245
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2246, _, _, _, _, _) => {
                            h2246
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2247, _, _, _, _) => {
                            h2247
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2248, _, _, _) => {
                            h2248
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2249, _, _) => {
                            h2249
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2250, _) => {
                            h2250
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2251) => {
                            h2251
                        }
                    })
                }
            } else {
                let a2252 = v1;
                let a2253 = 7u64;
                if nat_lt(a2252, a2253) {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2254, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2254
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2255, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2255
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2256, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2256
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2257, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2257
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2258, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2258
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2259, _, _, _, _, _, _, _, _, _, _) => {
                            h2259
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2260, _, _, _, _, _, _, _, _) => {
                            h2260
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2261, _, _, _, _, _, _, _) => {
                            h2261
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2262, _, _, _, _, _, _) => {
                            h2262
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2263, _, _, _, _, _) => {
                            h2263
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2264, _, _, _, _) => {
                            h2264
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2265, _, _, _) => {
                            h2265
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2266, _, _) => {
                            h2266
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2267, _) => {
                            h2267
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2268) => {
                            h2268
                        }
                    })
                } else {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2269, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2269
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2270, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2270
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2271, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2271
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2272, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2272
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2273, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2273
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2274, _, _, _, _, _, _, _, _, _, _) => {
                            h2274
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2275, _, _, _, _, _, _, _, _, _) => {
                            h2275
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2276, _, _, _, _, _, _, _) => {
                            h2276
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2277, _, _, _, _, _, _) => {
                            h2277
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2278, _, _, _, _, _) => {
                            h2278
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2279, _, _, _, _) => {
                            h2279
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2280, _, _, _) => {
                            h2280
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2281, _, _) => {
                            h2281
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2282, _) => {
                            h2282
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2283) => {
                            h2283
                        }
                    })
                }
            }
        }
    } else {
        let a2284 = v1;
        let a2285 = 12u64;
        if nat_lt(a2284, a2285) {
            let a2286 = v1;
            let a2287 = 10u64;
            if nat_lt(a2286, a2287) {
                let a2288 = v1;
                let a2289 = 9u64;
                if nat_lt(a2288, a2289) {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2290, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2290
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2291, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2291
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2292, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2292
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2293, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2293
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2294, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2294
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2295, _, _, _, _, _, _, _, _, _, _) => {
                            h2295
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2296, _, _, _, _, _, _, _, _, _) => {
                            h2296
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2297, _, _, _, _, _, _, _, _) => {
                            h2297
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2298, _, _, _, _, _, _) => {
                            h2298
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2299, _, _, _, _, _) => {
                            h2299
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2300, _, _, _, _) => {
                            h2300
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2301, _, _, _) => {
                            h2301
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2302, _, _) => {
                            h2302
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2303, _) => {
                            h2303
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2304) => {
                            h2304
                        }
                    })
                } else {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2305, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2305
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2306, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2306
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2307, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2307
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2308, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2308
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2309, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2309
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2310, _, _, _, _, _, _, _, _, _, _) => {
                            h2310
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2311, _, _, _, _, _, _, _, _, _) => {
                            h2311
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2312, _, _, _, _, _, _, _, _) => {
                            h2312
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2313, _, _, _, _, _, _, _) => {
                            h2313
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2314, _, _, _, _, _) => {
                            h2314
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2315, _, _, _, _) => {
                            h2315
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2316, _, _, _) => {
                            h2316
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2317, _, _) => {
                            h2317
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2318, _) => {
                            h2318
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2319) => {
                            h2319
                        }
                    })
                }
            } else {
                let a2320 = v1;
                let a2321 = 11u64;
                if nat_lt(a2320, a2321) {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2322, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2322
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2323, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2323
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2324, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2324
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2325, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2325
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2326, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2326
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2327, _, _, _, _, _, _, _, _, _, _) => {
                            h2327
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2328, _, _, _, _, _, _, _, _, _) => {
                            h2328
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2329, _, _, _, _, _, _, _, _) => {
                            h2329
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2330, _, _, _, _, _, _, _) => {
                            h2330
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2331, _, _, _, _, _, _) => {
                            h2331
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2332, _, _, _, _) => {
                            h2332
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2333, _, _, _) => {
                            h2333
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2334, _, _) => {
                            h2334
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2335, _) => {
                            h2335
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2336) => {
                            h2336
                        }
                    })
                } else {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2337, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2337
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2338, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2338
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2339, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2339
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2340, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2340
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2341, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2341
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2342, _, _, _, _, _, _, _, _, _, _) => {
                            h2342
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2343, _, _, _, _, _, _, _, _, _) => {
                            h2343
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2344, _, _, _, _, _, _, _, _) => {
                            h2344
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2345, _, _, _, _, _, _, _) => {
                            h2345
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2346, _, _, _, _, _, _) => {
                            h2346
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2347, _, _, _, _, _) => {
                            h2347
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2348, _, _, _) => {
                            h2348
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2349, _, _) => {
                            h2349
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2350, _) => {
                            h2350
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2351) => {
                            h2351
                        }
                    })
                }
            }
        } else {
            let a2352 = v1;
            let a2353 = 14u64;
            if nat_lt(a2352, a2353) {
                let a2354 = v1;
                let a2355 = 13u64;
                if nat_lt(a2354, a2355) {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2356, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2356
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2357, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2357
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2358, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2358
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2359, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2359
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2360, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2360
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2361, _, _, _, _, _, _, _, _, _, _) => {
                            h2361
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2362, _, _, _, _, _, _, _, _, _) => {
                            h2362
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2363, _, _, _, _, _, _, _, _) => {
                            h2363
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2364, _, _, _, _, _, _, _) => {
                            h2364
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2365, _, _, _, _, _, _) => {
                            h2365
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2366, _, _, _, _, _) => {
                            h2366
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2367, _, _, _, _) => {
                            h2367
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2368, _, _) => {
                            h2368
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2369, _) => {
                            h2369
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2370) => {
                            h2370
                        }
                    })
                } else {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2371, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2371
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2372, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2372
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2373, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2373
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2374, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2374
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2375, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2375
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2376, _, _, _, _, _, _, _, _, _, _) => {
                            h2376
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2377, _, _, _, _, _, _, _, _, _) => {
                            h2377
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2378, _, _, _, _, _, _, _, _) => {
                            h2378
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2379, _, _, _, _, _, _, _) => {
                            h2379
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2380, _, _, _, _, _, _) => {
                            h2380
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2381, _, _, _, _, _) => {
                            h2381
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2382, _, _, _, _) => {
                            h2382
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2383, _, _, _) => {
                            h2383
                        }
                    }, v2, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2384, _) => {
                            h2384
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2385) => {
                            h2385
                        }
                    })
                }
            } else {
                let a2386 = v1;
                let a2387 = 15u64;
                if nat_lt(a2386, a2387) {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2388, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2388
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2389, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2389
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2390, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2390
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2391, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2391
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2392, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2392
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2393, _, _, _, _, _, _, _, _, _, _) => {
                            h2393
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2394, _, _, _, _, _, _, _, _, _) => {
                            h2394
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2395, _, _, _, _, _, _, _, _) => {
                            h2395
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2396, _, _, _, _, _, _, _) => {
                            h2396
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2397, _, _, _, _, _, _) => {
                            h2397
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2398, _, _, _, _, _) => {
                            h2398
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2399, _, _, _, _) => {
                            h2399
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2400, _, _, _) => {
                            h2400
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2401, _, _) => {
                            h2401
                        }
                    }, v2, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2402) => {
                            h2402
                        }
                    })
                } else {
                    Adt6::C0(match v0.clone() {
                        Adt6::C0(h2403, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2403
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, h2404, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2404
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, h2405, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2405
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, h2406, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2406
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, h2407, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2407
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, h2408, _, _, _, _, _, _, _, _, _, _) => {
                            h2408
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, h2409, _, _, _, _, _, _, _, _, _) => {
                            h2409
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, h2410, _, _, _, _, _, _, _, _) => {
                            h2410
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2411, _, _, _, _, _, _, _) => {
                            h2411
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2412, _, _, _, _, _, _) => {
                            h2412
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2413, _, _, _, _, _) => {
                            h2413
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2414, _, _, _, _) => {
                            h2414
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2415, _, _, _) => {
                            h2415
                        }
                    }, match v0.clone() {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2416, _, _) => {
                            h2416
                        }
                    }, match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2417, _) => {
                            h2417
                        }
                    }, v2)
                }
            }
        }
    }
}

pub fn f123(v0: Adt18) -> R<Adt18> {
    let v1: u32 = f137(match v0.clone() {
        Adt18::C0(_, _, _, _, _, _, _, _, _, h2418) => {
            h2418
        }
    });
    let v2: u32 = f116(match v0.clone() {
        Adt18::C0(_, _, _, _, _, _, _, h2419, _, _) => {
            h2419
        }
    }, f116(f138(match v0.clone() {
        Adt18::C0(_, _, _, _, h2420, _, _, _, _, _) => {
            h2420
        }
    }), f116(f139(match v0.clone() {
        Adt18::C0(_, _, _, _, h2421, _, _, _, _, _) => {
            h2421
        }
    }, match v0.clone() {
        Adt18::C0(_, _, _, _, _, h2422, _, _, _, _) => {
            h2422
        }
    }, match v0.clone() {
        Adt18::C0(_, _, _, _, _, _, h2423, _, _, _) => {
            h2423
        }
    }), f116(v1, {
        let m2426 = match v0.clone() {
            Adt18::C0(_, _, _, _, _, _, _, _, h2425, _) => {
                h2425
            }
        };
        match m2426 {
            Adt17::C0(h2424, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                h2424
            }
        }
    }))));
    let v3: u32 = f116(f140(match v0.clone() {
        Adt18::C0(h2427, _, _, _, _, _, _, _, _, _) => {
            h2427
        }
    }), f141(match v0.clone() {
        Adt18::C0(h2428, _, _, _, _, _, _, _, _, _) => {
            h2428
        }
    }, match v0.clone() {
        Adt18::C0(_, h2429, _, _, _, _, _, _, _, _) => {
            h2429
        }
    }, match v0.clone() {
        Adt18::C0(_, _, h2430, _, _, _, _, _, _, _) => {
            h2430
        }
    }));
    Ok(Adt18::C0(f116(v2, v3), match v0.clone() {
        Adt18::C0(h2431, _, _, _, _, _, _, _, _, _) => {
            h2431
        }
    }, match v0.clone() {
        Adt18::C0(_, h2432, _, _, _, _, _, _, _, _) => {
            h2432
        }
    }, match v0.clone() {
        Adt18::C0(_, _, h2433, _, _, _, _, _, _, _) => {
            h2433
        }
    }, f116(match v0.clone() {
        Adt18::C0(_, _, _, h2434, _, _, _, _, _, _) => {
            h2434
        }
    }, v2), match v0.clone() {
        Adt18::C0(_, _, _, _, h2435, _, _, _, _, _) => {
            h2435
        }
    }, match v0.clone() {
        Adt18::C0(_, _, _, _, _, h2436, _, _, _, _) => {
            h2436
        }
    }, match v0.clone() {
        Adt18::C0(_, _, _, _, _, _, h2437, _, _, _) => {
            h2437
        }
    }, f142(match v0.clone() {
        Adt18::C0(_, _, _, _, _, _, _, _, h2438, _) => {
            h2438
        }
    }), {
        let a2440 = match v0 {
            Adt18::C0(_, _, _, _, _, _, _, _, _, h2439) => {
                h2439
            }
        };
        let a2441 = 1u64;
        nat_add(a2440, a2441)?
    }))
}

pub fn f124(v0: u32) -> u64 {
    let m2443 = {
        let a2442 = v0;
        fixed_u64::convert(i128::from(a2442))
    };
    match m2443 {
        None => {
            0u64
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f125(v0: u64) -> u32 {
    let m2447 = {
        let a2446 = {
            let a2444 = v0;
            let a2445 = 4294967295u64;
            fixed_u64::bit_and(a2444, a2445)
        };
        fixed_u32::convert(i128::from(a2446))
    };
    match m2447 {
        None => {
            0u32
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f126(v0: u64) -> u32 {
    let (_, h2448) = {
        let (_, h2449) = f92(Fn13::F143, 32u64, (v0, (1u32, 0u32)));
        h2449
    };
    h2448
}

pub fn f127(v0: u32, v1: u32, v2: u32, v3: u32) -> u32 {
    let a2454 = f144(v0, 24u32);
    let a2455 = {
        let a2452 = f144(v1, 16u32);
        let a2453 = {
            let a2450 = f144(v2, 8u32);
            let a2451 = v3;
            fixed_u32::bit_or(a2450, a2451)
        };
        fixed_u32::bit_or(a2452, a2453)
    };
    fixed_u32::bit_or(a2454, a2455)
}

pub fn f128(v0: Bytes, v1: u64, v2: u64) -> u32 {
    let a2456 = v2;
    let a2457 = v1;
    if nat_lt(a2456, a2457) {
        let m2460 = {
            let a2458 = v0;
            let a2459 = v2;
            index_bytes(a2458, a2459)
        };
        match m2460 {
            None => {
                0u32
            }
            Some(v3) => {
                let m2462 = {
                    let a2461 = v3;
                    fixed_u32::convert(i128::from(a2461))
                };
                match m2462 {
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
        let a2463 = v2;
        let a2464 = v1;
        if nat_eq(a2463, a2464) {
            128u32
        } else {
            0u32
        }
    }
}

pub fn f129(v0: u64) -> Adt19 {
    let a2465 = v0;
    let a2466 = 64u64;
    if nat_lt(a2465, a2466) {
        let a2467 = v0;
        let a2468 = 32u64;
        if nat_lt(a2467, a2468) {
            let a2469 = v0;
            let a2470 = 16u64;
            if nat_lt(a2469, a2470) {
                let a2471 = v0;
                let a2472 = 8u64;
                if nat_lt(a2471, a2472) {
                    let a2473 = v0;
                    let a2474 = 4u64;
                    if nat_lt(a2473, a2474) {
                        let a2475 = v0;
                        let a2476 = 2u64;
                        if nat_lt(a2475, a2476) {
                            let a2477 = v0;
                            let a2478 = 1u64;
                            if nat_lt(a2477, a2478) {
                                Adt19::C0(0u64, 0u64, Adt15::C0, 0u64, 4u64)
                            } else {
                                Adt19::C0(0u64, 1u64, Adt15::C1, 0u64, 0u64)
                            }
                        } else {
                            let a2479 = v0;
                            let a2480 = 3u64;
                            if nat_lt(a2479, a2480) {
                                Adt19::C0(0u64, 2u64, Adt15::C2, 12u64, 0u64)
                            } else {
                                Adt19::C0(0u64, 3u64, Adt15::C3, 12u64, 16u64)
                            }
                        }
                    } else {
                        let a2481 = v0;
                        let a2482 = 6u64;
                        if nat_lt(a2481, a2482) {
                            let a2483 = v0;
                            let a2484 = 5u64;
                            if nat_lt(a2483, a2484) {
                                Adt19::C0(0u64, 4u64, Adt15::C0, 8u64, 12u64)
                            } else {
                                Adt19::C0(0u64, 5u64, Adt15::C2, 4u64, 8u64)
                            }
                        } else {
                            let a2485 = v0;
                            let a2486 = 7u64;
                            if nat_lt(a2485, a2486) {
                                Adt19::C0(0u64, 6u64, Adt15::C3, 4u64, 12u64)
                            } else {
                                Adt19::C0(0u64, 7u64, Adt15::C0, 0u64, 4u64)
                            }
                        }
                    }
                } else {
                    let a2487 = v0;
                    let a2488 = 12u64;
                    if nat_lt(a2487, a2488) {
                        let a2489 = v0;
                        let a2490 = 10u64;
                        if nat_lt(a2489, a2490) {
                            let a2491 = v0;
                            let a2492 = 9u64;
                            if nat_lt(a2491, a2492) {
                                Adt19::C0(0u64, 8u64, Adt15::C1, 0u64, 1u64)
                            } else {
                                Adt19::C0(0u64, 9u64, Adt15::C2, 12u64, 0u64)
                            }
                        } else {
                            let a2493 = v0;
                            let a2494 = 11u64;
                            if nat_lt(a2493, a2494) {
                                Adt19::C0(0u64, 10u64, Adt15::C3, 12u64, 8u64)
                            } else {
                                Adt19::C0(0u64, 11u64, Adt15::C0, 8u64, 12u64)
                            }
                        }
                    } else {
                        let a2495 = v0;
                        let a2496 = 14u64;
                        if nat_lt(a2495, a2496) {
                            let a2497 = v0;
                            let a2498 = 13u64;
                            if nat_lt(a2497, a2498) {
                                Adt19::C0(0u64, 12u64, Adt15::C2, 4u64, 8u64)
                            } else {
                                Adt19::C0(0u64, 13u64, Adt15::C3, 4u64, 7u64)
                            }
                        } else {
                            let a2499 = v0;
                            let a2500 = 15u64;
                            if nat_lt(a2499, a2500) {
                                Adt19::C0(1u64, 0u64, Adt15::C0, 1u64, 5u64)
                            } else {
                                Adt19::C0(1u64, 1u64, Adt15::C1, 1u64, 2u64)
                            }
                        }
                    }
                }
            } else {
                let a2501 = v0;
                let a2502 = 24u64;
                if nat_lt(a2501, a2502) {
                    let a2503 = v0;
                    let a2504 = 20u64;
                    if nat_lt(a2503, a2504) {
                        let a2505 = v0;
                        let a2506 = 18u64;
                        if nat_lt(a2505, a2506) {
                            let a2507 = v0;
                            let a2508 = 17u64;
                            if nat_lt(a2507, a2508) {
                                Adt19::C0(1u64, 2u64, Adt15::C2, 13u64, 1u64)
                            } else {
                                Adt19::C0(1u64, 3u64, Adt15::C3, 13u64, 16u64)
                            }
                        } else {
                            let a2509 = v0;
                            let a2510 = 19u64;
                            if nat_lt(a2509, a2510) {
                                Adt19::C0(1u64, 4u64, Adt15::C0, 9u64, 13u64)
                            } else {
                                Adt19::C0(1u64, 5u64, Adt15::C2, 5u64, 9u64)
                            }
                        }
                    } else {
                        let a2511 = v0;
                        let a2512 = 22u64;
                        if nat_lt(a2511, a2512) {
                            let a2513 = v0;
                            let a2514 = 21u64;
                            if nat_lt(a2513, a2514) {
                                Adt19::C0(1u64, 6u64, Adt15::C3, 5u64, 12u64)
                            } else {
                                Adt19::C0(1u64, 7u64, Adt15::C0, 1u64, 5u64)
                            }
                        } else {
                            let a2515 = v0;
                            let a2516 = 23u64;
                            if nat_lt(a2515, a2516) {
                                Adt19::C0(1u64, 8u64, Adt15::C1, 1u64, 3u64)
                            } else {
                                Adt19::C0(1u64, 9u64, Adt15::C2, 13u64, 1u64)
                            }
                        }
                    }
                } else {
                    let a2517 = v0;
                    let a2518 = 28u64;
                    if nat_lt(a2517, a2518) {
                        let a2519 = v0;
                        let a2520 = 26u64;
                        if nat_lt(a2519, a2520) {
                            let a2521 = v0;
                            let a2522 = 25u64;
                            if nat_lt(a2521, a2522) {
                                Adt19::C0(1u64, 10u64, Adt15::C3, 13u64, 8u64)
                            } else {
                                Adt19::C0(1u64, 11u64, Adt15::C0, 9u64, 13u64)
                            }
                        } else {
                            let a2523 = v0;
                            let a2524 = 27u64;
                            if nat_lt(a2523, a2524) {
                                Adt19::C0(1u64, 12u64, Adt15::C2, 5u64, 9u64)
                            } else {
                                Adt19::C0(1u64, 13u64, Adt15::C3, 5u64, 7u64)
                            }
                        }
                    } else {
                        let a2525 = v0;
                        let a2526 = 30u64;
                        if nat_lt(a2525, a2526) {
                            let a2527 = v0;
                            let a2528 = 29u64;
                            if nat_lt(a2527, a2528) {
                                Adt19::C0(2u64, 0u64, Adt15::C0, 2u64, 6u64)
                            } else {
                                Adt19::C0(2u64, 1u64, Adt15::C1, 2u64, 4u64)
                            }
                        } else {
                            let a2529 = v0;
                            let a2530 = 31u64;
                            if nat_lt(a2529, a2530) {
                                Adt19::C0(2u64, 2u64, Adt15::C2, 14u64, 2u64)
                            } else {
                                Adt19::C0(2u64, 3u64, Adt15::C3, 14u64, 16u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a2531 = v0;
            let a2532 = 48u64;
            if nat_lt(a2531, a2532) {
                let a2533 = v0;
                let a2534 = 40u64;
                if nat_lt(a2533, a2534) {
                    let a2535 = v0;
                    let a2536 = 36u64;
                    if nat_lt(a2535, a2536) {
                        let a2537 = v0;
                        let a2538 = 34u64;
                        if nat_lt(a2537, a2538) {
                            let a2539 = v0;
                            let a2540 = 33u64;
                            if nat_lt(a2539, a2540) {
                                Adt19::C0(2u64, 4u64, Adt15::C0, 10u64, 14u64)
                            } else {
                                Adt19::C0(2u64, 5u64, Adt15::C2, 6u64, 10u64)
                            }
                        } else {
                            let a2541 = v0;
                            let a2542 = 35u64;
                            if nat_lt(a2541, a2542) {
                                Adt19::C0(2u64, 6u64, Adt15::C3, 6u64, 12u64)
                            } else {
                                Adt19::C0(2u64, 7u64, Adt15::C0, 2u64, 6u64)
                            }
                        }
                    } else {
                        let a2543 = v0;
                        let a2544 = 38u64;
                        if nat_lt(a2543, a2544) {
                            let a2545 = v0;
                            let a2546 = 37u64;
                            if nat_lt(a2545, a2546) {
                                Adt19::C0(2u64, 8u64, Adt15::C1, 2u64, 5u64)
                            } else {
                                Adt19::C0(2u64, 9u64, Adt15::C2, 14u64, 2u64)
                            }
                        } else {
                            let a2547 = v0;
                            let a2548 = 39u64;
                            if nat_lt(a2547, a2548) {
                                Adt19::C0(2u64, 10u64, Adt15::C3, 14u64, 8u64)
                            } else {
                                Adt19::C0(2u64, 11u64, Adt15::C0, 10u64, 14u64)
                            }
                        }
                    }
                } else {
                    let a2549 = v0;
                    let a2550 = 44u64;
                    if nat_lt(a2549, a2550) {
                        let a2551 = v0;
                        let a2552 = 42u64;
                        if nat_lt(a2551, a2552) {
                            let a2553 = v0;
                            let a2554 = 41u64;
                            if nat_lt(a2553, a2554) {
                                Adt19::C0(2u64, 12u64, Adt15::C2, 6u64, 10u64)
                            } else {
                                Adt19::C0(2u64, 13u64, Adt15::C3, 6u64, 7u64)
                            }
                        } else {
                            let a2555 = v0;
                            let a2556 = 43u64;
                            if nat_lt(a2555, a2556) {
                                Adt19::C0(3u64, 0u64, Adt15::C0, 3u64, 7u64)
                            } else {
                                Adt19::C0(3u64, 1u64, Adt15::C1, 3u64, 6u64)
                            }
                        }
                    } else {
                        let a2557 = v0;
                        let a2558 = 46u64;
                        if nat_lt(a2557, a2558) {
                            let a2559 = v0;
                            let a2560 = 45u64;
                            if nat_lt(a2559, a2560) {
                                Adt19::C0(3u64, 2u64, Adt15::C2, 15u64, 3u64)
                            } else {
                                Adt19::C0(3u64, 3u64, Adt15::C3, 15u64, 16u64)
                            }
                        } else {
                            let a2561 = v0;
                            let a2562 = 47u64;
                            if nat_lt(a2561, a2562) {
                                Adt19::C0(3u64, 4u64, Adt15::C0, 11u64, 15u64)
                            } else {
                                Adt19::C0(3u64, 5u64, Adt15::C2, 7u64, 11u64)
                            }
                        }
                    }
                }
            } else {
                let a2563 = v0;
                let a2564 = 56u64;
                if nat_lt(a2563, a2564) {
                    let a2565 = v0;
                    let a2566 = 52u64;
                    if nat_lt(a2565, a2566) {
                        let a2567 = v0;
                        let a2568 = 50u64;
                        if nat_lt(a2567, a2568) {
                            let a2569 = v0;
                            let a2570 = 49u64;
                            if nat_lt(a2569, a2570) {
                                Adt19::C0(3u64, 6u64, Adt15::C3, 7u64, 12u64)
                            } else {
                                Adt19::C0(3u64, 7u64, Adt15::C0, 3u64, 7u64)
                            }
                        } else {
                            let a2571 = v0;
                            let a2572 = 51u64;
                            if nat_lt(a2571, a2572) {
                                Adt19::C0(3u64, 8u64, Adt15::C1, 3u64, 7u64)
                            } else {
                                Adt19::C0(3u64, 9u64, Adt15::C2, 15u64, 3u64)
                            }
                        }
                    } else {
                        let a2573 = v0;
                        let a2574 = 54u64;
                        if nat_lt(a2573, a2574) {
                            let a2575 = v0;
                            let a2576 = 53u64;
                            if nat_lt(a2575, a2576) {
                                Adt19::C0(3u64, 10u64, Adt15::C3, 15u64, 8u64)
                            } else {
                                Adt19::C0(3u64, 11u64, Adt15::C0, 11u64, 15u64)
                            }
                        } else {
                            let a2577 = v0;
                            let a2578 = 55u64;
                            if nat_lt(a2577, a2578) {
                                Adt19::C0(3u64, 12u64, Adt15::C2, 7u64, 11u64)
                            } else {
                                Adt19::C0(3u64, 13u64, Adt15::C3, 7u64, 7u64)
                            }
                        }
                    }
                } else {
                    let a2579 = v0;
                    let a2580 = 60u64;
                    if nat_lt(a2579, a2580) {
                        let a2581 = v0;
                        let a2582 = 58u64;
                        if nat_lt(a2581, a2582) {
                            let a2583 = v0;
                            let a2584 = 57u64;
                            if nat_lt(a2583, a2584) {
                                Adt19::C0(4u64, 0u64, Adt15::C0, 0u64, 5u64)
                            } else {
                                Adt19::C0(4u64, 1u64, Adt15::C1, 0u64, 8u64)
                            }
                        } else {
                            let a2585 = v0;
                            let a2586 = 59u64;
                            if nat_lt(a2585, a2586) {
                                Adt19::C0(4u64, 2u64, Adt15::C2, 15u64, 0u64)
                            } else {
                                Adt19::C0(4u64, 3u64, Adt15::C3, 15u64, 16u64)
                            }
                        }
                    } else {
                        let a2587 = v0;
                        let a2588 = 62u64;
                        if nat_lt(a2587, a2588) {
                            let a2589 = v0;
                            let a2590 = 61u64;
                            if nat_lt(a2589, a2590) {
                                Adt19::C0(4u64, 4u64, Adt15::C0, 10u64, 15u64)
                            } else {
                                Adt19::C0(4u64, 5u64, Adt15::C2, 5u64, 10u64)
                            }
                        } else {
                            let a2591 = v0;
                            let a2592 = 63u64;
                            if nat_lt(a2591, a2592) {
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
        let a2593 = v0;
        let a2594 = 96u64;
        if nat_lt(a2593, a2594) {
            let a2595 = v0;
            let a2596 = 80u64;
            if nat_lt(a2595, a2596) {
                let a2597 = v0;
                let a2598 = 72u64;
                if nat_lt(a2597, a2598) {
                    let a2599 = v0;
                    let a2600 = 68u64;
                    if nat_lt(a2599, a2600) {
                        let a2601 = v0;
                        let a2602 = 66u64;
                        if nat_lt(a2601, a2602) {
                            let a2603 = v0;
                            let a2604 = 65u64;
                            if nat_lt(a2603, a2604) {
                                Adt19::C0(4u64, 8u64, Adt15::C1, 0u64, 9u64)
                            } else {
                                Adt19::C0(4u64, 9u64, Adt15::C2, 15u64, 0u64)
                            }
                        } else {
                            let a2605 = v0;
                            let a2606 = 67u64;
                            if nat_lt(a2605, a2606) {
                                Adt19::C0(4u64, 10u64, Adt15::C3, 15u64, 8u64)
                            } else {
                                Adt19::C0(4u64, 11u64, Adt15::C0, 10u64, 15u64)
                            }
                        }
                    } else {
                        let a2607 = v0;
                        let a2608 = 70u64;
                        if nat_lt(a2607, a2608) {
                            let a2609 = v0;
                            let a2610 = 69u64;
                            if nat_lt(a2609, a2610) {
                                Adt19::C0(4u64, 12u64, Adt15::C2, 5u64, 10u64)
                            } else {
                                Adt19::C0(4u64, 13u64, Adt15::C3, 5u64, 7u64)
                            }
                        } else {
                            let a2611 = v0;
                            let a2612 = 71u64;
                            if nat_lt(a2611, a2612) {
                                Adt19::C0(5u64, 0u64, Adt15::C0, 1u64, 6u64)
                            } else {
                                Adt19::C0(5u64, 1u64, Adt15::C1, 1u64, 10u64)
                            }
                        }
                    }
                } else {
                    let a2613 = v0;
                    let a2614 = 76u64;
                    if nat_lt(a2613, a2614) {
                        let a2615 = v0;
                        let a2616 = 74u64;
                        if nat_lt(a2615, a2616) {
                            let a2617 = v0;
                            let a2618 = 73u64;
                            if nat_lt(a2617, a2618) {
                                Adt19::C0(5u64, 2u64, Adt15::C2, 12u64, 1u64)
                            } else {
                                Adt19::C0(5u64, 3u64, Adt15::C3, 12u64, 16u64)
                            }
                        } else {
                            let a2619 = v0;
                            let a2620 = 75u64;
                            if nat_lt(a2619, a2620) {
                                Adt19::C0(5u64, 4u64, Adt15::C0, 11u64, 12u64)
                            } else {
                                Adt19::C0(5u64, 5u64, Adt15::C2, 6u64, 11u64)
                            }
                        }
                    } else {
                        let a2621 = v0;
                        let a2622 = 78u64;
                        if nat_lt(a2621, a2622) {
                            let a2623 = v0;
                            let a2624 = 77u64;
                            if nat_lt(a2623, a2624) {
                                Adt19::C0(5u64, 6u64, Adt15::C3, 6u64, 12u64)
                            } else {
                                Adt19::C0(5u64, 7u64, Adt15::C0, 1u64, 6u64)
                            }
                        } else {
                            let a2625 = v0;
                            let a2626 = 79u64;
                            if nat_lt(a2625, a2626) {
                                Adt19::C0(5u64, 8u64, Adt15::C1, 1u64, 11u64)
                            } else {
                                Adt19::C0(5u64, 9u64, Adt15::C2, 12u64, 1u64)
                            }
                        }
                    }
                }
            } else {
                let a2627 = v0;
                let a2628 = 88u64;
                if nat_lt(a2627, a2628) {
                    let a2629 = v0;
                    let a2630 = 84u64;
                    if nat_lt(a2629, a2630) {
                        let a2631 = v0;
                        let a2632 = 82u64;
                        if nat_lt(a2631, a2632) {
                            let a2633 = v0;
                            let a2634 = 81u64;
                            if nat_lt(a2633, a2634) {
                                Adt19::C0(5u64, 10u64, Adt15::C3, 12u64, 8u64)
                            } else {
                                Adt19::C0(5u64, 11u64, Adt15::C0, 11u64, 12u64)
                            }
                        } else {
                            let a2635 = v0;
                            let a2636 = 83u64;
                            if nat_lt(a2635, a2636) {
                                Adt19::C0(5u64, 12u64, Adt15::C2, 6u64, 11u64)
                            } else {
                                Adt19::C0(5u64, 13u64, Adt15::C3, 6u64, 7u64)
                            }
                        }
                    } else {
                        let a2637 = v0;
                        let a2638 = 86u64;
                        if nat_lt(a2637, a2638) {
                            let a2639 = v0;
                            let a2640 = 85u64;
                            if nat_lt(a2639, a2640) {
                                Adt19::C0(6u64, 0u64, Adt15::C0, 2u64, 7u64)
                            } else {
                                Adt19::C0(6u64, 1u64, Adt15::C1, 2u64, 12u64)
                            }
                        } else {
                            let a2641 = v0;
                            let a2642 = 87u64;
                            if nat_lt(a2641, a2642) {
                                Adt19::C0(6u64, 2u64, Adt15::C2, 13u64, 2u64)
                            } else {
                                Adt19::C0(6u64, 3u64, Adt15::C3, 13u64, 16u64)
                            }
                        }
                    }
                } else {
                    let a2643 = v0;
                    let a2644 = 92u64;
                    if nat_lt(a2643, a2644) {
                        let a2645 = v0;
                        let a2646 = 90u64;
                        if nat_lt(a2645, a2646) {
                            let a2647 = v0;
                            let a2648 = 89u64;
                            if nat_lt(a2647, a2648) {
                                Adt19::C0(6u64, 4u64, Adt15::C0, 8u64, 13u64)
                            } else {
                                Adt19::C0(6u64, 5u64, Adt15::C2, 7u64, 8u64)
                            }
                        } else {
                            let a2649 = v0;
                            let a2650 = 91u64;
                            if nat_lt(a2649, a2650) {
                                Adt19::C0(6u64, 6u64, Adt15::C3, 7u64, 12u64)
                            } else {
                                Adt19::C0(6u64, 7u64, Adt15::C0, 2u64, 7u64)
                            }
                        }
                    } else {
                        let a2651 = v0;
                        let a2652 = 94u64;
                        if nat_lt(a2651, a2652) {
                            let a2653 = v0;
                            let a2654 = 93u64;
                            if nat_lt(a2653, a2654) {
                                Adt19::C0(6u64, 8u64, Adt15::C1, 2u64, 13u64)
                            } else {
                                Adt19::C0(6u64, 9u64, Adt15::C2, 13u64, 2u64)
                            }
                        } else {
                            let a2655 = v0;
                            let a2656 = 95u64;
                            if nat_lt(a2655, a2656) {
                                Adt19::C0(6u64, 10u64, Adt15::C3, 13u64, 8u64)
                            } else {
                                Adt19::C0(6u64, 11u64, Adt15::C0, 8u64, 13u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a2657 = v0;
            let a2658 = 112u64;
            if nat_lt(a2657, a2658) {
                let a2659 = v0;
                let a2660 = 104u64;
                if nat_lt(a2659, a2660) {
                    let a2661 = v0;
                    let a2662 = 100u64;
                    if nat_lt(a2661, a2662) {
                        let a2663 = v0;
                        let a2664 = 98u64;
                        if nat_lt(a2663, a2664) {
                            let a2665 = v0;
                            let a2666 = 97u64;
                            if nat_lt(a2665, a2666) {
                                Adt19::C0(6u64, 12u64, Adt15::C2, 7u64, 8u64)
                            } else {
                                Adt19::C0(6u64, 13u64, Adt15::C3, 7u64, 7u64)
                            }
                        } else {
                            let a2667 = v0;
                            let a2668 = 99u64;
                            if nat_lt(a2667, a2668) {
                                Adt19::C0(7u64, 0u64, Adt15::C0, 3u64, 4u64)
                            } else {
                                Adt19::C0(7u64, 1u64, Adt15::C1, 3u64, 14u64)
                            }
                        }
                    } else {
                        let a2669 = v0;
                        let a2670 = 102u64;
                        if nat_lt(a2669, a2670) {
                            let a2671 = v0;
                            let a2672 = 101u64;
                            if nat_lt(a2671, a2672) {
                                Adt19::C0(7u64, 2u64, Adt15::C2, 14u64, 3u64)
                            } else {
                                Adt19::C0(7u64, 3u64, Adt15::C3, 14u64, 16u64)
                            }
                        } else {
                            let a2673 = v0;
                            let a2674 = 103u64;
                            if nat_lt(a2673, a2674) {
                                Adt19::C0(7u64, 4u64, Adt15::C0, 9u64, 14u64)
                            } else {
                                Adt19::C0(7u64, 5u64, Adt15::C2, 4u64, 9u64)
                            }
                        }
                    }
                } else {
                    let a2675 = v0;
                    let a2676 = 108u64;
                    if nat_lt(a2675, a2676) {
                        let a2677 = v0;
                        let a2678 = 106u64;
                        if nat_lt(a2677, a2678) {
                            let a2679 = v0;
                            let a2680 = 105u64;
                            if nat_lt(a2679, a2680) {
                                Adt19::C0(7u64, 6u64, Adt15::C3, 4u64, 12u64)
                            } else {
                                Adt19::C0(7u64, 7u64, Adt15::C0, 3u64, 4u64)
                            }
                        } else {
                            let a2681 = v0;
                            let a2682 = 107u64;
                            if nat_lt(a2681, a2682) {
                                Adt19::C0(7u64, 8u64, Adt15::C1, 3u64, 15u64)
                            } else {
                                Adt19::C0(7u64, 9u64, Adt15::C2, 14u64, 3u64)
                            }
                        }
                    } else {
                        let a2683 = v0;
                        let a2684 = 110u64;
                        if nat_lt(a2683, a2684) {
                            let a2685 = v0;
                            let a2686 = 109u64;
                            if nat_lt(a2685, a2686) {
                                Adt19::C0(7u64, 10u64, Adt15::C3, 14u64, 8u64)
                            } else {
                                Adt19::C0(7u64, 11u64, Adt15::C0, 9u64, 14u64)
                            }
                        } else {
                            let a2687 = v0;
                            let a2688 = 111u64;
                            if nat_lt(a2687, a2688) {
                                Adt19::C0(7u64, 12u64, Adt15::C2, 4u64, 9u64)
                            } else {
                                Adt19::C0(7u64, 13u64, Adt15::C3, 4u64, 7u64)
                            }
                        }
                    }
                }
            } else {
                let a2689 = v0;
                let a2690 = 120u64;
                if nat_lt(a2689, a2690) {
                    let a2691 = v0;
                    let a2692 = 116u64;
                    if nat_lt(a2691, a2692) {
                        let a2693 = v0;
                        let a2694 = 114u64;
                        if nat_lt(a2693, a2694) {
                            let a2695 = v0;
                            let a2696 = 113u64;
                            if nat_lt(a2695, a2696) {
                                Adt19::C0(0u64, 0u64, Adt15::C2, 0u64, 8u64)
                            } else {
                                Adt19::C0(0u64, 1u64, Adt15::C2, 1u64, 9u64)
                            }
                        } else {
                            let a2697 = v0;
                            let a2698 = 115u64;
                            if nat_lt(a2697, a2698) {
                                Adt19::C0(0u64, 2u64, Adt15::C2, 2u64, 10u64)
                            } else {
                                Adt19::C0(0u64, 3u64, Adt15::C2, 3u64, 11u64)
                            }
                        }
                    } else {
                        let a2699 = v0;
                        let a2700 = 118u64;
                        if nat_lt(a2699, a2700) {
                            let a2701 = v0;
                            let a2702 = 117u64;
                            if nat_lt(a2701, a2702) {
                                Adt19::C0(0u64, 4u64, Adt15::C2, 4u64, 12u64)
                            } else {
                                Adt19::C0(0u64, 5u64, Adt15::C2, 5u64, 13u64)
                            }
                        } else {
                            let a2703 = v0;
                            let a2704 = 119u64;
                            if nat_lt(a2703, a2704) {
                                Adt19::C0(0u64, 6u64, Adt15::C2, 6u64, 14u64)
                            } else {
                                Adt19::C0(0u64, 7u64, Adt15::C2, 7u64, 15u64)
                            }
                        }
                    }
                } else {
                    let a2705 = v0;
                    let a2706 = 124u64;
                    if nat_lt(a2705, a2706) {
                        let a2707 = v0;
                        let a2708 = 122u64;
                        if nat_lt(a2707, a2708) {
                            let a2709 = v0;
                            let a2710 = 121u64;
                            if nat_lt(a2709, a2710) {
                                Adt19::C0(0u64, 8u64, Adt15::C4, 8u64, 0u64)
                            } else {
                                Adt19::C0(0u64, 9u64, Adt15::C4, 9u64, 1u64)
                            }
                        } else {
                            let a2711 = v0;
                            let a2712 = 123u64;
                            if nat_lt(a2711, a2712) {
                                Adt19::C0(0u64, 10u64, Adt15::C4, 10u64, 2u64)
                            } else {
                                Adt19::C0(0u64, 11u64, Adt15::C4, 11u64, 3u64)
                            }
                        }
                    } else {
                        let a2713 = v0;
                        let a2714 = 126u64;
                        if nat_lt(a2713, a2714) {
                            let a2715 = v0;
                            let a2716 = 125u64;
                            if nat_lt(a2715, a2716) {
                                Adt19::C0(0u64, 12u64, Adt15::C4, 12u64, 4u64)
                            } else {
                                Adt19::C0(0u64, 13u64, Adt15::C4, 13u64, 5u64)
                            }
                        } else {
                            let a2717 = v0;
                            let a2718 = 127u64;
                            if nat_lt(a2717, a2718) {
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
    let a2719 = v1;
    let a2720 = 8u64;
    if nat_lt(a2719, a2720) {
        let a2721 = v1;
        let a2722 = 4u64;
        if nat_lt(a2721, a2722) {
            let a2723 = v1;
            let a2724 = 2u64;
            if nat_lt(a2723, a2724) {
                let a2725 = v1;
                let a2726 = 1u64;
                if nat_lt(a2725, a2726) {
                    match v0 {
                        Adt6::C0(h2727, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2727
                        }
                    }
                } else {
                    match v0 {
                        Adt6::C0(_, h2728, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2728
                        }
                    }
                }
            } else {
                let a2729 = v1;
                let a2730 = 3u64;
                if nat_lt(a2729, a2730) {
                    match v0 {
                        Adt6::C0(_, _, h2731, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2731
                        }
                    }
                } else {
                    match v0 {
                        Adt6::C0(_, _, _, h2732, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2732
                        }
                    }
                }
            }
        } else {
            let a2733 = v1;
            let a2734 = 6u64;
            if nat_lt(a2733, a2734) {
                let a2735 = v1;
                let a2736 = 5u64;
                if nat_lt(a2735, a2736) {
                    match v0 {
                        Adt6::C0(_, _, _, _, h2737, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2737
                        }
                    }
                } else {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, h2738, _, _, _, _, _, _, _, _, _, _) => {
                            h2738
                        }
                    }
                }
            } else {
                let a2739 = v1;
                let a2740 = 7u64;
                if nat_lt(a2739, a2740) {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, h2741, _, _, _, _, _, _, _, _, _) => {
                            h2741
                        }
                    }
                } else {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, h2742, _, _, _, _, _, _, _, _) => {
                            h2742
                        }
                    }
                }
            }
        }
    } else {
        let a2743 = v1;
        let a2744 = 12u64;
        if nat_lt(a2743, a2744) {
            let a2745 = v1;
            let a2746 = 10u64;
            if nat_lt(a2745, a2746) {
                let a2747 = v1;
                let a2748 = 9u64;
                if nat_lt(a2747, a2748) {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, h2749, _, _, _, _, _, _, _) => {
                            h2749
                        }
                    }
                } else {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, h2750, _, _, _, _, _, _) => {
                            h2750
                        }
                    }
                }
            } else {
                let a2751 = v1;
                let a2752 = 11u64;
                if nat_lt(a2751, a2752) {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, h2753, _, _, _, _, _) => {
                            h2753
                        }
                    }
                } else {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, h2754, _, _, _, _) => {
                            h2754
                        }
                    }
                }
            }
        } else {
            let a2755 = v1;
            let a2756 = 14u64;
            if nat_lt(a2755, a2756) {
                let a2757 = v1;
                let a2758 = 13u64;
                if nat_lt(a2757, a2758) {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2759, _, _, _) => {
                            h2759
                        }
                    }
                } else {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2760, _, _) => {
                            h2760
                        }
                    }
                }
            } else {
                let a2761 = v1;
                let a2762 = 15u64;
                if nat_lt(a2761, a2762) {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2763, _) => {
                            h2763
                        }
                    }
                } else {
                    match v0 {
                        Adt6::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2764) => {
                            h2764
                        }
                    }
                }
            }
        }
    }
}

pub fn f131(v0: u64, v1: u64) -> R<u64> {
    let v2: u64 = {
        let a2767 = {
            let a2765 = v0;
            let a2766 = 16u64;
            nat_mul(a2765, a2766)?
        };
        let a2768 = v1;
        nat_add(a2767, a2768)?
    };
    let a2769 = v2;
    let a2770 = 56u64;
    if nat_lt(a2769, a2770) {
        let a2771 = v2;
        let a2772 = 28u64;
        if nat_lt(a2771, a2772) {
            let a2773 = v2;
            let a2774 = 14u64;
            if nat_lt(a2773, a2774) {
                let a2775 = v2;
                let a2776 = 7u64;
                if nat_lt(a2775, a2776) {
                    let a2777 = v2;
                    let a2778 = 3u64;
                    if nat_lt(a2777, a2778) {
                        let a2779 = v2;
                        let a2780 = 1u64;
                        if nat_lt(a2779, a2780) {
                            Ok(0u64)
                        } else {
                            let a2781 = v2;
                            let a2782 = 2u64;
                            if nat_lt(a2781, a2782) {
                                Ok(1u64)
                            } else {
                                Ok(2u64)
                            }
                        }
                    } else {
                        let a2783 = v2;
                        let a2784 = 5u64;
                        if nat_lt(a2783, a2784) {
                            let a2785 = v2;
                            let a2786 = 4u64;
                            if nat_lt(a2785, a2786) {
                                Ok(3u64)
                            } else {
                                Ok(4u64)
                            }
                        } else {
                            let a2787 = v2;
                            let a2788 = 6u64;
                            if nat_lt(a2787, a2788) {
                                Ok(5u64)
                            } else {
                                Ok(6u64)
                            }
                        }
                    }
                } else {
                    let a2789 = v2;
                    let a2790 = 10u64;
                    if nat_lt(a2789, a2790) {
                        let a2791 = v2;
                        let a2792 = 8u64;
                        if nat_lt(a2791, a2792) {
                            Ok(7u64)
                        } else {
                            let a2793 = v2;
                            let a2794 = 9u64;
                            if nat_lt(a2793, a2794) {
                                Ok(8u64)
                            } else {
                                Ok(9u64)
                            }
                        }
                    } else {
                        let a2795 = v2;
                        let a2796 = 12u64;
                        if nat_lt(a2795, a2796) {
                            let a2797 = v2;
                            let a2798 = 11u64;
                            if nat_lt(a2797, a2798) {
                                Ok(10u64)
                            } else {
                                Ok(11u64)
                            }
                        } else {
                            let a2799 = v2;
                            let a2800 = 13u64;
                            if nat_lt(a2799, a2800) {
                                Ok(12u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    }
                }
            } else {
                let a2801 = v2;
                let a2802 = 21u64;
                if nat_lt(a2801, a2802) {
                    let a2803 = v2;
                    let a2804 = 17u64;
                    if nat_lt(a2803, a2804) {
                        let a2805 = v2;
                        let a2806 = 15u64;
                        if nat_lt(a2805, a2806) {
                            Ok(14u64)
                        } else {
                            let a2807 = v2;
                            let a2808 = 16u64;
                            if nat_lt(a2807, a2808) {
                                Ok(15u64)
                            } else {
                                Ok(2u64)
                            }
                        }
                    } else {
                        let a2809 = v2;
                        let a2810 = 19u64;
                        if nat_lt(a2809, a2810) {
                            let a2811 = v2;
                            let a2812 = 18u64;
                            if nat_lt(a2811, a2812) {
                                Ok(6u64)
                            } else {
                                Ok(3u64)
                            }
                        } else {
                            let a2813 = v2;
                            let a2814 = 20u64;
                            if nat_lt(a2813, a2814) {
                                Ok(10u64)
                            } else {
                                Ok(7u64)
                            }
                        }
                    }
                } else {
                    let a2815 = v2;
                    let a2816 = 24u64;
                    if nat_lt(a2815, a2816) {
                        let a2817 = v2;
                        let a2818 = 22u64;
                        if nat_lt(a2817, a2818) {
                            Ok(0u64)
                        } else {
                            let a2819 = v2;
                            let a2820 = 23u64;
                            if nat_lt(a2819, a2820) {
                                Ok(4u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    } else {
                        let a2821 = v2;
                        let a2822 = 26u64;
                        if nat_lt(a2821, a2822) {
                            let a2823 = v2;
                            let a2824 = 25u64;
                            if nat_lt(a2823, a2824) {
                                Ok(1u64)
                            } else {
                                Ok(11u64)
                            }
                        } else {
                            let a2825 = v2;
                            let a2826 = 27u64;
                            if nat_lt(a2825, a2826) {
                                Ok(12u64)
                            } else {
                                Ok(5u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a2827 = v2;
            let a2828 = 42u64;
            if nat_lt(a2827, a2828) {
                let a2829 = v2;
                let a2830 = 35u64;
                if nat_lt(a2829, a2830) {
                    let a2831 = v2;
                    let a2832 = 31u64;
                    if nat_lt(a2831, a2832) {
                        let a2833 = v2;
                        let a2834 = 29u64;
                        if nat_lt(a2833, a2834) {
                            Ok(9u64)
                        } else {
                            let a2835 = v2;
                            let a2836 = 30u64;
                            if nat_lt(a2835, a2836) {
                                Ok(14u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    } else {
                        let a2837 = v2;
                        let a2838 = 33u64;
                        if nat_lt(a2837, a2838) {
                            let a2839 = v2;
                            let a2840 = 32u64;
                            if nat_lt(a2839, a2840) {
                                Ok(8u64)
                            } else {
                                Ok(3u64)
                            }
                        } else {
                            let a2841 = v2;
                            let a2842 = 34u64;
                            if nat_lt(a2841, a2842) {
                                Ok(4u64)
                            } else {
                                Ok(10u64)
                            }
                        }
                    }
                } else {
                    let a2843 = v2;
                    let a2844 = 38u64;
                    if nat_lt(a2843, a2844) {
                        let a2845 = v2;
                        let a2846 = 36u64;
                        if nat_lt(a2845, a2846) {
                            Ok(12u64)
                        } else {
                            let a2847 = v2;
                            let a2848 = 37u64;
                            if nat_lt(a2847, a2848) {
                                Ok(13u64)
                            } else {
                                Ok(2u64)
                            }
                        }
                    } else {
                        let a2849 = v2;
                        let a2850 = 40u64;
                        if nat_lt(a2849, a2850) {
                            let a2851 = v2;
                            let a2852 = 39u64;
                            if nat_lt(a2851, a2852) {
                                Ok(7u64)
                            } else {
                                Ok(14u64)
                            }
                        } else {
                            let a2853 = v2;
                            let a2854 = 41u64;
                            if nat_lt(a2853, a2854) {
                                Ok(6u64)
                            } else {
                                Ok(5u64)
                            }
                        }
                    }
                }
            } else {
                let a2855 = v2;
                let a2856 = 49u64;
                if nat_lt(a2855, a2856) {
                    let a2857 = v2;
                    let a2858 = 45u64;
                    if nat_lt(a2857, a2858) {
                        let a2859 = v2;
                        let a2860 = 43u64;
                        if nat_lt(a2859, a2860) {
                            Ok(9u64)
                        } else {
                            let a2861 = v2;
                            let a2862 = 44u64;
                            if nat_lt(a2861, a2862) {
                                Ok(0u64)
                            } else {
                                Ok(11u64)
                            }
                        }
                    } else {
                        let a2863 = v2;
                        let a2864 = 47u64;
                        if nat_lt(a2863, a2864) {
                            let a2865 = v2;
                            let a2866 = 46u64;
                            if nat_lt(a2865, a2866) {
                                Ok(15u64)
                            } else {
                                Ok(8u64)
                            }
                        } else {
                            let a2867 = v2;
                            let a2868 = 48u64;
                            if nat_lt(a2867, a2868) {
                                Ok(1u64)
                            } else {
                                Ok(10u64)
                            }
                        }
                    }
                } else {
                    let a2869 = v2;
                    let a2870 = 52u64;
                    if nat_lt(a2869, a2870) {
                        let a2871 = v2;
                        let a2872 = 50u64;
                        if nat_lt(a2871, a2872) {
                            Ok(7u64)
                        } else {
                            let a2873 = v2;
                            let a2874 = 51u64;
                            if nat_lt(a2873, a2874) {
                                Ok(12u64)
                            } else {
                                Ok(9u64)
                            }
                        }
                    } else {
                        let a2875 = v2;
                        let a2876 = 54u64;
                        if nat_lt(a2875, a2876) {
                            let a2877 = v2;
                            let a2878 = 53u64;
                            if nat_lt(a2877, a2878) {
                                Ok(14u64)
                            } else {
                                Ok(3u64)
                            }
                        } else {
                            let a2879 = v2;
                            let a2880 = 55u64;
                            if nat_lt(a2879, a2880) {
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
        let a2881 = v2;
        let a2882 = 84u64;
        if nat_lt(a2881, a2882) {
            let a2883 = v2;
            let a2884 = 70u64;
            if nat_lt(a2883, a2884) {
                let a2885 = v2;
                let a2886 = 63u64;
                if nat_lt(a2885, a2886) {
                    let a2887 = v2;
                    let a2888 = 59u64;
                    if nat_lt(a2887, a2888) {
                        let a2889 = v2;
                        let a2890 = 57u64;
                        if nat_lt(a2889, a2890) {
                            Ok(4u64)
                        } else {
                            let a2891 = v2;
                            let a2892 = 58u64;
                            if nat_lt(a2891, a2892) {
                                Ok(0u64)
                            } else {
                                Ok(11u64)
                            }
                        }
                    } else {
                        let a2893 = v2;
                        let a2894 = 61u64;
                        if nat_lt(a2893, a2894) {
                            let a2895 = v2;
                            let a2896 = 60u64;
                            if nat_lt(a2895, a2896) {
                                Ok(2u64)
                            } else {
                                Ok(5u64)
                            }
                        } else {
                            let a2897 = v2;
                            let a2898 = 62u64;
                            if nat_lt(a2897, a2898) {
                                Ok(8u64)
                            } else {
                                Ok(1u64)
                            }
                        }
                    }
                } else {
                    let a2899 = v2;
                    let a2900 = 66u64;
                    if nat_lt(a2899, a2900) {
                        let a2901 = v2;
                        let a2902 = 64u64;
                        if nat_lt(a2901, a2902) {
                            Ok(6u64)
                        } else {
                            let a2903 = v2;
                            let a2904 = 65u64;
                            if nat_lt(a2903, a2904) {
                                Ok(12u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    } else {
                        let a2905 = v2;
                        let a2906 = 68u64;
                        if nat_lt(a2905, a2906) {
                            let a2907 = v2;
                            let a2908 = 67u64;
                            if nat_lt(a2907, a2908) {
                                Ok(9u64)
                            } else {
                                Ok(11u64)
                            }
                        } else {
                            let a2909 = v2;
                            let a2910 = 69u64;
                            if nat_lt(a2909, a2910) {
                                Ok(15u64)
                            } else {
                                Ok(10u64)
                            }
                        }
                    }
                }
            } else {
                let a2911 = v2;
                let a2912 = 77u64;
                if nat_lt(a2911, a2912) {
                    let a2913 = v2;
                    let a2914 = 73u64;
                    if nat_lt(a2913, a2914) {
                        let a2915 = v2;
                        let a2916 = 71u64;
                        if nat_lt(a2915, a2916) {
                            Ok(14u64)
                        } else {
                            let a2917 = v2;
                            let a2918 = 72u64;
                            if nat_lt(a2917, a2918) {
                                Ok(8u64)
                            } else {
                                Ok(7u64)
                            }
                        }
                    } else {
                        let a2919 = v2;
                        let a2920 = 75u64;
                        if nat_lt(a2919, a2920) {
                            let a2921 = v2;
                            let a2922 = 74u64;
                            if nat_lt(a2921, a2922) {
                                Ok(2u64)
                            } else {
                                Ok(5u64)
                            }
                        } else {
                            let a2923 = v2;
                            let a2924 = 76u64;
                            if nat_lt(a2923, a2924) {
                                Ok(3u64)
                            } else {
                                Ok(0u64)
                            }
                        }
                    }
                } else {
                    let a2925 = v2;
                    let a2926 = 80u64;
                    if nat_lt(a2925, a2926) {
                        let a2927 = v2;
                        let a2928 = 78u64;
                        if nat_lt(a2927, a2928) {
                            Ok(1u64)
                        } else {
                            let a2929 = v2;
                            let a2930 = 79u64;
                            if nat_lt(a2929, a2930) {
                                Ok(6u64)
                            } else {
                                Ok(4u64)
                            }
                        }
                    } else {
                        let a2931 = v2;
                        let a2932 = 82u64;
                        if nat_lt(a2931, a2932) {
                            let a2933 = v2;
                            let a2934 = 81u64;
                            if nat_lt(a2933, a2934) {
                                Ok(9u64)
                            } else {
                                Ok(14u64)
                            }
                        } else {
                            let a2935 = v2;
                            let a2936 = 83u64;
                            if nat_lt(a2935, a2936) {
                                Ok(11u64)
                            } else {
                                Ok(5u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a2937 = v2;
            let a2938 = 98u64;
            if nat_lt(a2937, a2938) {
                let a2939 = v2;
                let a2940 = 91u64;
                if nat_lt(a2939, a2940) {
                    let a2941 = v2;
                    let a2942 = 87u64;
                    if nat_lt(a2941, a2942) {
                        let a2943 = v2;
                        let a2944 = 85u64;
                        if nat_lt(a2943, a2944) {
                            Ok(8u64)
                        } else {
                            let a2945 = v2;
                            let a2946 = 86u64;
                            if nat_lt(a2945, a2946) {
                                Ok(12u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    } else {
                        let a2947 = v2;
                        let a2948 = 89u64;
                        if nat_lt(a2947, a2948) {
                            let a2949 = v2;
                            let a2950 = 88u64;
                            if nat_lt(a2949, a2950) {
                                Ok(1u64)
                            } else {
                                Ok(13u64)
                            }
                        } else {
                            let a2951 = v2;
                            let a2952 = 90u64;
                            if nat_lt(a2951, a2952) {
                                Ok(3u64)
                            } else {
                                Ok(0u64)
                            }
                        }
                    }
                } else {
                    let a2953 = v2;
                    let a2954 = 94u64;
                    if nat_lt(a2953, a2954) {
                        let a2955 = v2;
                        let a2956 = 92u64;
                        if nat_lt(a2955, a2956) {
                            Ok(10u64)
                        } else {
                            let a2957 = v2;
                            let a2958 = 93u64;
                            if nat_lt(a2957, a2958) {
                                Ok(2u64)
                            } else {
                                Ok(6u64)
                            }
                        }
                    } else {
                        let a2959 = v2;
                        let a2960 = 96u64;
                        if nat_lt(a2959, a2960) {
                            let a2961 = v2;
                            let a2962 = 95u64;
                            if nat_lt(a2961, a2962) {
                                Ok(4u64)
                            } else {
                                Ok(7u64)
                            }
                        } else {
                            let a2963 = v2;
                            let a2964 = 97u64;
                            if nat_lt(a2963, a2964) {
                                Ok(11u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    }
                }
            } else {
                let a2965 = v2;
                let a2966 = 105u64;
                if nat_lt(a2965, a2966) {
                    let a2967 = v2;
                    let a2968 = 101u64;
                    if nat_lt(a2967, a2968) {
                        let a2969 = v2;
                        let a2970 = 99u64;
                        if nat_lt(a2969, a2970) {
                            Ok(5u64)
                        } else {
                            let a2971 = v2;
                            let a2972 = 100u64;
                            if nat_lt(a2971, a2972) {
                                Ok(0u64)
                            } else {
                                Ok(1u64)
                            }
                        }
                    } else {
                        let a2973 = v2;
                        let a2974 = 103u64;
                        if nat_lt(a2973, a2974) {
                            let a2975 = v2;
                            let a2976 = 102u64;
                            if nat_lt(a2975, a2976) {
                                Ok(9u64)
                            } else {
                                Ok(8u64)
                            }
                        } else {
                            let a2977 = v2;
                            let a2978 = 104u64;
                            if nat_lt(a2977, a2978) {
                                Ok(6u64)
                            } else {
                                Ok(14u64)
                            }
                        }
                    }
                } else {
                    let a2979 = v2;
                    let a2980 = 108u64;
                    if nat_lt(a2979, a2980) {
                        let a2981 = v2;
                        let a2982 = 106u64;
                        if nat_lt(a2981, a2982) {
                            Ok(10u64)
                        } else {
                            let a2983 = v2;
                            let a2984 = 107u64;
                            if nat_lt(a2983, a2984) {
                                Ok(2u64)
                            } else {
                                Ok(12u64)
                            }
                        }
                    } else {
                        let a2985 = v2;
                        let a2986 = 110u64;
                        if nat_lt(a2985, a2986) {
                            let a2987 = v2;
                            let a2988 = 109u64;
                            if nat_lt(a2987, a2988) {
                                Ok(3u64)
                            } else {
                                Ok(4u64)
                            }
                        } else {
                            let a2989 = v2;
                            let a2990 = 111u64;
                            if nat_lt(a2989, a2990) {
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
    let a2991 = v0;
    let a2992 = 12u64;
    if nat_lt(a2991, a2992) {
        let a2993 = v0;
        let a2994 = 9u64;
        if nat_lt(a2993, a2994) {
            let a2995 = v0;
            let a2996 = 8u64;
            if nat_lt(a2995, a2996) {
                7u32
            } else {
                8u32
            }
        } else {
            let a2997 = v0;
            let a2998 = 10u64;
            if nat_lt(a2997, a2998) {
                9u32
            } else {
                let a2999 = v0;
                let a3000 = 11u64;
                if nat_lt(a2999, a3000) {
                    10u32
                } else {
                    11u32
                }
            }
        }
    } else {
        let a3001 = v0;
        let a3002 = 14u64;
        if nat_lt(a3001, a3002) {
            let a3003 = v0;
            let a3004 = 13u64;
            if nat_lt(a3003, a3004) {
                12u32
            } else {
                13u32
            }
        } else {
            let a3005 = v0;
            let a3006 = 15u64;
            if nat_lt(a3005, a3006) {
                14u32
            } else {
                let a3007 = v0;
                let a3008 = 16u64;
                if nat_lt(a3007, a3008) {
                    15u32
                } else {
                    16u32
                }
            }
        }
    }
}

pub fn f133(v0: Adt5, v1: u64) -> u32 {
    let a3009 = v1;
    let a3010 = 4u64;
    if nat_lt(a3009, a3010) {
        let a3011 = v1;
        let a3012 = 2u64;
        if nat_lt(a3011, a3012) {
            let a3013 = v1;
            let a3014 = 1u64;
            if nat_lt(a3013, a3014) {
                match v0 {
                    Adt5::C0(h3015, _, _, _, _, _, _, _) => {
                        h3015
                    }
                }
            } else {
                match v0 {
                    Adt5::C0(_, h3016, _, _, _, _, _, _) => {
                        h3016
                    }
                }
            }
        } else {
            let a3017 = v1;
            let a3018 = 3u64;
            if nat_lt(a3017, a3018) {
                match v0 {
                    Adt5::C0(_, _, h3019, _, _, _, _, _) => {
                        h3019
                    }
                }
            } else {
                match v0 {
                    Adt5::C0(_, _, _, h3020, _, _, _, _) => {
                        h3020
                    }
                }
            }
        }
    } else {
        let a3021 = v1;
        let a3022 = 6u64;
        if nat_lt(a3021, a3022) {
            let a3023 = v1;
            let a3024 = 5u64;
            if nat_lt(a3023, a3024) {
                match v0 {
                    Adt5::C0(_, _, _, _, h3025, _, _, _) => {
                        h3025
                    }
                }
            } else {
                match v0 {
                    Adt5::C0(_, _, _, _, _, h3026, _, _) => {
                        h3026
                    }
                }
            }
        } else {
            let a3027 = v1;
            let a3028 = 7u64;
            if nat_lt(a3027, a3028) {
                match v0 {
                    Adt5::C0(_, _, _, _, _, _, h3029, _) => {
                        h3029
                    }
                }
            } else {
                match v0 {
                    Adt5::C0(_, _, _, _, _, _, _, h3030) => {
                        h3030
                    }
                }
            }
        }
    }
}

pub fn f134(v0: Adt15, v1: u32, v2: u32) -> u32 {
    let m3031 = v0;
    match m3031 {
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
    let (_, h3032) = {
        let (_, h3033) = f148(Fn19::F149(v0, v1), 32u64, (0u32, (0u32, Bytes::lit(&[]))));
        h3033
    };
    h3032
}

pub fn f136(v0: u32, v1: u32) -> Bytes {
    let (_, h3034) = f150(Fn20::F151(v0, v1), 32u64, (0u32, Bytes::lit(&[])));
    h3034
}

pub fn f137(v0: u64) -> u32 {
    let a3035 = v0;
    let a3036 = 32u64;
    if nat_lt(a3035, a3036) {
        let a3037 = v0;
        let a3038 = 16u64;
        if nat_lt(a3037, a3038) {
            let a3039 = v0;
            let a3040 = 8u64;
            if nat_lt(a3039, a3040) {
                let a3041 = v0;
                let a3042 = 4u64;
                if nat_lt(a3041, a3042) {
                    let a3043 = v0;
                    let a3044 = 2u64;
                    if nat_lt(a3043, a3044) {
                        let a3045 = v0;
                        let a3046 = 1u64;
                        if nat_lt(a3045, a3046) {
                            1116352408u32
                        } else {
                            1899447441u32
                        }
                    } else {
                        let a3047 = v0;
                        let a3048 = 3u64;
                        if nat_lt(a3047, a3048) {
                            3049323471u32
                        } else {
                            3921009573u32
                        }
                    }
                } else {
                    let a3049 = v0;
                    let a3050 = 6u64;
                    if nat_lt(a3049, a3050) {
                        let a3051 = v0;
                        let a3052 = 5u64;
                        if nat_lt(a3051, a3052) {
                            961987163u32
                        } else {
                            1508970993u32
                        }
                    } else {
                        let a3053 = v0;
                        let a3054 = 7u64;
                        if nat_lt(a3053, a3054) {
                            2453635748u32
                        } else {
                            2870763221u32
                        }
                    }
                }
            } else {
                let a3055 = v0;
                let a3056 = 12u64;
                if nat_lt(a3055, a3056) {
                    let a3057 = v0;
                    let a3058 = 10u64;
                    if nat_lt(a3057, a3058) {
                        let a3059 = v0;
                        let a3060 = 9u64;
                        if nat_lt(a3059, a3060) {
                            3624381080u32
                        } else {
                            310598401u32
                        }
                    } else {
                        let a3061 = v0;
                        let a3062 = 11u64;
                        if nat_lt(a3061, a3062) {
                            607225278u32
                        } else {
                            1426881987u32
                        }
                    }
                } else {
                    let a3063 = v0;
                    let a3064 = 14u64;
                    if nat_lt(a3063, a3064) {
                        let a3065 = v0;
                        let a3066 = 13u64;
                        if nat_lt(a3065, a3066) {
                            1925078388u32
                        } else {
                            2162078206u32
                        }
                    } else {
                        let a3067 = v0;
                        let a3068 = 15u64;
                        if nat_lt(a3067, a3068) {
                            2614888103u32
                        } else {
                            3248222580u32
                        }
                    }
                }
            }
        } else {
            let a3069 = v0;
            let a3070 = 24u64;
            if nat_lt(a3069, a3070) {
                let a3071 = v0;
                let a3072 = 20u64;
                if nat_lt(a3071, a3072) {
                    let a3073 = v0;
                    let a3074 = 18u64;
                    if nat_lt(a3073, a3074) {
                        let a3075 = v0;
                        let a3076 = 17u64;
                        if nat_lt(a3075, a3076) {
                            3835390401u32
                        } else {
                            4022224774u32
                        }
                    } else {
                        let a3077 = v0;
                        let a3078 = 19u64;
                        if nat_lt(a3077, a3078) {
                            264347078u32
                        } else {
                            604807628u32
                        }
                    }
                } else {
                    let a3079 = v0;
                    let a3080 = 22u64;
                    if nat_lt(a3079, a3080) {
                        let a3081 = v0;
                        let a3082 = 21u64;
                        if nat_lt(a3081, a3082) {
                            770255983u32
                        } else {
                            1249150122u32
                        }
                    } else {
                        let a3083 = v0;
                        let a3084 = 23u64;
                        if nat_lt(a3083, a3084) {
                            1555081692u32
                        } else {
                            1996064986u32
                        }
                    }
                }
            } else {
                let a3085 = v0;
                let a3086 = 28u64;
                if nat_lt(a3085, a3086) {
                    let a3087 = v0;
                    let a3088 = 26u64;
                    if nat_lt(a3087, a3088) {
                        let a3089 = v0;
                        let a3090 = 25u64;
                        if nat_lt(a3089, a3090) {
                            2554220882u32
                        } else {
                            2821834349u32
                        }
                    } else {
                        let a3091 = v0;
                        let a3092 = 27u64;
                        if nat_lt(a3091, a3092) {
                            2952996808u32
                        } else {
                            3210313671u32
                        }
                    }
                } else {
                    let a3093 = v0;
                    let a3094 = 30u64;
                    if nat_lt(a3093, a3094) {
                        let a3095 = v0;
                        let a3096 = 29u64;
                        if nat_lt(a3095, a3096) {
                            3336571891u32
                        } else {
                            3584528711u32
                        }
                    } else {
                        let a3097 = v0;
                        let a3098 = 31u64;
                        if nat_lt(a3097, a3098) {
                            113926993u32
                        } else {
                            338241895u32
                        }
                    }
                }
            }
        }
    } else {
        let a3099 = v0;
        let a3100 = 48u64;
        if nat_lt(a3099, a3100) {
            let a3101 = v0;
            let a3102 = 40u64;
            if nat_lt(a3101, a3102) {
                let a3103 = v0;
                let a3104 = 36u64;
                if nat_lt(a3103, a3104) {
                    let a3105 = v0;
                    let a3106 = 34u64;
                    if nat_lt(a3105, a3106) {
                        let a3107 = v0;
                        let a3108 = 33u64;
                        if nat_lt(a3107, a3108) {
                            666307205u32
                        } else {
                            773529912u32
                        }
                    } else {
                        let a3109 = v0;
                        let a3110 = 35u64;
                        if nat_lt(a3109, a3110) {
                            1294757372u32
                        } else {
                            1396182291u32
                        }
                    }
                } else {
                    let a3111 = v0;
                    let a3112 = 38u64;
                    if nat_lt(a3111, a3112) {
                        let a3113 = v0;
                        let a3114 = 37u64;
                        if nat_lt(a3113, a3114) {
                            1695183700u32
                        } else {
                            1986661051u32
                        }
                    } else {
                        let a3115 = v0;
                        let a3116 = 39u64;
                        if nat_lt(a3115, a3116) {
                            2177026350u32
                        } else {
                            2456956037u32
                        }
                    }
                }
            } else {
                let a3117 = v0;
                let a3118 = 44u64;
                if nat_lt(a3117, a3118) {
                    let a3119 = v0;
                    let a3120 = 42u64;
                    if nat_lt(a3119, a3120) {
                        let a3121 = v0;
                        let a3122 = 41u64;
                        if nat_lt(a3121, a3122) {
                            2730485921u32
                        } else {
                            2820302411u32
                        }
                    } else {
                        let a3123 = v0;
                        let a3124 = 43u64;
                        if nat_lt(a3123, a3124) {
                            3259730800u32
                        } else {
                            3345764771u32
                        }
                    }
                } else {
                    let a3125 = v0;
                    let a3126 = 46u64;
                    if nat_lt(a3125, a3126) {
                        let a3127 = v0;
                        let a3128 = 45u64;
                        if nat_lt(a3127, a3128) {
                            3516065817u32
                        } else {
                            3600352804u32
                        }
                    } else {
                        let a3129 = v0;
                        let a3130 = 47u64;
                        if nat_lt(a3129, a3130) {
                            4094571909u32
                        } else {
                            275423344u32
                        }
                    }
                }
            }
        } else {
            let a3131 = v0;
            let a3132 = 56u64;
            if nat_lt(a3131, a3132) {
                let a3133 = v0;
                let a3134 = 52u64;
                if nat_lt(a3133, a3134) {
                    let a3135 = v0;
                    let a3136 = 50u64;
                    if nat_lt(a3135, a3136) {
                        let a3137 = v0;
                        let a3138 = 49u64;
                        if nat_lt(a3137, a3138) {
                            430227734u32
                        } else {
                            506948616u32
                        }
                    } else {
                        let a3139 = v0;
                        let a3140 = 51u64;
                        if nat_lt(a3139, a3140) {
                            659060556u32
                        } else {
                            883997877u32
                        }
                    }
                } else {
                    let a3141 = v0;
                    let a3142 = 54u64;
                    if nat_lt(a3141, a3142) {
                        let a3143 = v0;
                        let a3144 = 53u64;
                        if nat_lt(a3143, a3144) {
                            958139571u32
                        } else {
                            1322822218u32
                        }
                    } else {
                        let a3145 = v0;
                        let a3146 = 55u64;
                        if nat_lt(a3145, a3146) {
                            1537002063u32
                        } else {
                            1747873779u32
                        }
                    }
                }
            } else {
                let a3147 = v0;
                let a3148 = 60u64;
                if nat_lt(a3147, a3148) {
                    let a3149 = v0;
                    let a3150 = 58u64;
                    if nat_lt(a3149, a3150) {
                        let a3151 = v0;
                        let a3152 = 57u64;
                        if nat_lt(a3151, a3152) {
                            1955562222u32
                        } else {
                            2024104815u32
                        }
                    } else {
                        let a3153 = v0;
                        let a3154 = 59u64;
                        if nat_lt(a3153, a3154) {
                            2227730452u32
                        } else {
                            2361852424u32
                        }
                    }
                } else {
                    let a3155 = v0;
                    let a3156 = 62u64;
                    if nat_lt(a3155, a3156) {
                        let a3157 = v0;
                        let a3158 = 61u64;
                        if nat_lt(a3157, a3158) {
                            2428436474u32
                        } else {
                            2756734187u32
                        }
                    } else {
                        let a3159 = v0;
                        let a3160 = 63u64;
                        if nat_lt(a3159, a3160) {
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
    let a3163 = {
        let a3161 = f152(v0, 6u32, 26u32);
        let a3162 = f152(v0, 11u32, 21u32);
        fixed_u32::bit_xor(a3161, a3162)
    };
    let a3164 = f152(v0, 25u32, 7u32);
    fixed_u32::bit_xor(a3163, a3164)
}

pub fn f139(v0: u32, v1: u32, v2: u32) -> u32 {
    let a3170 = {
        let a3165 = v0;
        let a3166 = v1;
        fixed_u32::bit_and(a3165, a3166)
    };
    let a3171 = {
        let a3168 = {
            let a3167 = v0;
            fixed_u32::bit_not(a3167)
        };
        let a3169 = v2;
        fixed_u32::bit_and(a3168, a3169)
    };
    fixed_u32::bit_xor(a3170, a3171)
}

pub fn f140(v0: u32) -> u32 {
    let a3174 = {
        let a3172 = f152(v0, 2u32, 30u32);
        let a3173 = f152(v0, 13u32, 19u32);
        fixed_u32::bit_xor(a3172, a3173)
    };
    let a3175 = f152(v0, 22u32, 10u32);
    fixed_u32::bit_xor(a3174, a3175)
}

pub fn f141(v0: u32, v1: u32, v2: u32) -> u32 {
    let a3184 = {
        let a3180 = {
            let a3176 = v0;
            let a3177 = v1;
            fixed_u32::bit_and(a3176, a3177)
        };
        let a3181 = {
            let a3178 = v0;
            let a3179 = v2;
            fixed_u32::bit_and(a3178, a3179)
        };
        fixed_u32::bit_xor(a3180, a3181)
    };
    let a3185 = {
        let a3182 = v1;
        let a3183 = v2;
        fixed_u32::bit_and(a3182, a3183)
    };
    fixed_u32::bit_xor(a3184, a3185)
}

pub fn f142(v0: Adt17) -> Adt17 {
    Adt17::C0(match v0.clone() {
        Adt17::C0(_, h3186, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h3186
        }
    }, match v0.clone() {
        Adt17::C0(_, _, h3187, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h3187
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, h3188, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h3188
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, _, h3189, _, _, _, _, _, _, _, _, _, _, _) => {
            h3189
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, _, _, h3190, _, _, _, _, _, _, _, _, _, _) => {
            h3190
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, _, _, _, h3191, _, _, _, _, _, _, _, _, _) => {
            h3191
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, _, _, _, _, h3192, _, _, _, _, _, _, _, _) => {
            h3192
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, _, _, _, _, _, h3193, _, _, _, _, _, _, _) => {
            h3193
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, _, _, _, _, _, _, h3194, _, _, _, _, _, _) => {
            h3194
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, _, _, _, _, _, _, _, h3195, _, _, _, _, _) => {
            h3195
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, _, _, _, _, _, _, _, _, h3196, _, _, _, _) => {
            h3196
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, _, _, _, _, _, _, _, _, _, h3197, _, _, _) => {
            h3197
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h3198, _, _) => {
            h3198
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h3199, _) => {
            h3199
        }
    }, match v0.clone() {
        Adt17::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h3200) => {
            h3200
        }
    }, f116(f153(match v0.clone() {
        Adt17::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h3201, _) => {
            h3201
        }
    }), f116(match v0.clone() {
        Adt17::C0(_, _, _, _, _, _, _, _, _, h3202, _, _, _, _, _, _) => {
            h3202
        }
    }, f116(f154(match v0.clone() {
        Adt17::C0(_, h3203, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h3203
        }
    }), match v0 {
        Adt17::C0(h3204, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h3204
        }
    }))))
}

pub fn f143(v0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let v1: u64 = {
        let (h3205, _) = v0;
        h3205
    };
    let v2: u32 = {
        let (h3206, _) = {
            let (_, h3207) = v0;
            h3207
        };
        h3206
    };
    let v3: u32 = {
        let (_, h3208) = {
            let (_, h3209) = v0;
            h3209
        };
        h3208
    };
    ({
        let a3210 = v1;
        let a3211 = 2u64;
        let a3212 = 0u64;
        nat_quot(a3210, a3211, a3212)
    }, (f144(v2, 1u32), {
        let a3216 = {
            let a3213 = v1;
            let a3214 = 2u64;
            let a3215 = 0u64;
            nat_rem(a3213, a3214, a3215)
        };
        let a3217 = 1u64;
        if nat_eq(a3216, a3217) {
            let a3218 = v3;
            let a3219 = v2;
            fixed_u32::bit_or(a3218, a3219)
        } else {
            v3
        }
    }))
}

pub fn f144(v0: u32, v1: u32) -> u32 {
    let m3222 = {
        let a3220 = v0;
        let a3221 = v1;
        fixed_u32::shift_left(a3220, a3221)
    };
    match m3222 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f145(v0: u32, v1: u32) -> u32 {
    let m3225 = {
        let a3223 = f155(v0);
        let a3224 = f155(v1);
        fixed_u64::checked_add(a3223, a3224)
    };
    match m3225 {
        None => {
            0u32
        }
        Some(v2) => {
            f156(v2)
        }
    }
}

pub fn f146(v0: u32, v1: u32) -> u32 {
    let a3226 = v0;
    let a3227 = v1;
    fixed_u32::bit_xor(a3226, a3227)
}

pub fn f147(v0: u32, v1: u32) -> u32 {
    let m3230 = {
        let a3228 = 32u32;
        let a3229 = v1;
        fixed_u32::checked_sub(a3228, a3229)
    };
    match m3230 {
        None => {
            v0
        }
        Some(v2) => {
            f157(v0, v1, v2)
        }
    }
}

pub fn f148(v0: Fn19, p3234: u64, p3235: (u32, (u32, Bytes))) -> (u32, (u32, Bytes)) {
    let m3233 = (p3234, p3235);
    {
        let mut m3233 = m3233;
        loop {
            let step = {
                let (v1, v2) = m3233;
                let m3231 = v1;
                if m3231 == 0 {
                    Err::<(u64, (u32, (u32, Bytes))), (u32, (u32, Bytes))>(v2)
                } else {
                    let v3 = m3231 - 1;
                    Ok::<(u64, (u32, (u32, Bytes))), (u32, (u32, Bytes))>((v3, {
                        let c3232: Fn19 = v0.clone();
                        c3232.apply(v2)
                    }))
                }
            };
            match step {
                Ok(next) => { m3233 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f149(v0: u32, v1: u32, v2: (u32, (u32, Bytes))) -> (u32, (u32, Bytes)) {
    let v3: u32 = {
        let (h3236, _) = v2.clone();
        h3236
    };
    let v4: u32 = f158(v0, v3);
    let v5: u32 = f158(v1, v3);
    let v6: u32 = {
        let (h3237, _) = {
            let (_, h3238) = v2.clone();
            h3238
        };
        h3237
    };
    (f145(v3, 1u32), (f159(v4, v5, v6), {
        let a3241 = {
            let (_, h3239) = {
                let (_, h3240) = v2;
                h3240
            };
            h3239
        };
        let a3242 = f160(f161(v4, v5, v6));
        append_bytes(a3241, a3242)
    }))
}

pub fn f150(v0: Fn20, p3246: u64, p3247: (u32, Bytes)) -> (u32, Bytes) {
    let m3245 = (p3246, p3247);
    {
        let mut m3245 = m3245;
        loop {
            let step = {
                let (v1, v2) = m3245;
                let m3243 = v1;
                if m3243 == 0 {
                    Err::<(u64, (u32, Bytes)), (u32, Bytes)>(v2)
                } else {
                    let v3 = m3243 - 1;
                    Ok::<(u64, (u32, Bytes)), (u32, Bytes)>((v3, {
                        let c3244: Fn20 = v0.clone();
                        c3244.apply(v2)
                    }))
                }
            };
            match step {
                Ok(next) => { m3245 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f151(v0: u32, v1: u32, v2: (u32, Bytes)) -> (u32, Bytes) {
    let v3: u32 = {
        let (h3248, _) = v2;
        h3248
    };
    (f145(v3, 1u32), {
        let a3250 = {
            let (_, h3249) = v2;
            h3249
        };
        let a3251 = f160(f162(f158(v0, v3), f158(v1, v3)));
        append_bytes(a3250, a3251)
    })
}

pub fn f152(v0: u32, v1: u32, v2: u32) -> u32 {
    let a3252 = f163(v0, v1);
    let a3253 = f144(v0, v2);
    fixed_u32::bit_or(a3252, a3253)
}

pub fn f153(v0: u32) -> u32 {
    let a3256 = {
        let a3254 = f152(v0, 17u32, 15u32);
        let a3255 = f152(v0, 19u32, 13u32);
        fixed_u32::bit_xor(a3254, a3255)
    };
    let a3257 = f163(v0, 10u32);
    fixed_u32::bit_xor(a3256, a3257)
}

pub fn f154(v0: u32) -> u32 {
    let a3260 = {
        let a3258 = f152(v0, 7u32, 25u32);
        let a3259 = f152(v0, 18u32, 14u32);
        fixed_u32::bit_xor(a3258, a3259)
    };
    let a3261 = f163(v0, 3u32);
    fixed_u32::bit_xor(a3260, a3261)
}

pub fn f155(v0: u32) -> u64 {
    let m3263 = {
        let a3262 = v0;
        fixed_u64::convert(i128::from(a3262))
    };
    match m3263 {
        None => {
            0u64
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f156(v0: u64) -> u32 {
    let m3267 = {
        let a3266 = {
            let a3264 = v0;
            let a3265 = 4294967295u64;
            fixed_u64::bit_and(a3264, a3265)
        };
        fixed_u32::convert(i128::from(a3266))
    };
    match m3267 {
        None => {
            0u32
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f157(v0: u32, v1: u32, v2: u32) -> u32 {
    let a3268 = f82(v0, v1);
    let a3269 = f105(v0, v2);
    fixed_u32::bit_or(a3268, a3269)
}

pub fn f158(v0: u32, v1: u32) -> u32 {
    let a3270 = f82(v0, v1);
    let a3271 = 1u32;
    fixed_u32::bit_and(a3270, a3271)
}

pub fn f159(v0: u32, v1: u32, v2: u32) -> u32 {
    let a3280 = {
        let a3276 = {
            let a3272 = v0;
            let a3273 = v1;
            fixed_u32::bit_and(a3272, a3273)
        };
        let a3277 = {
            let a3274 = v0;
            let a3275 = v2;
            fixed_u32::bit_and(a3274, a3275)
        };
        fixed_u32::bit_or(a3276, a3277)
    };
    let a3281 = {
        let a3278 = v1;
        let a3279 = v2;
        fixed_u32::bit_and(a3278, a3279)
    };
    fixed_u32::bit_or(a3280, a3281)
}

pub fn f160(v0: u32) -> Bytes {
    let a3284 = {
        let a3282 = v0;
        let a3283 = 16u32;
        fixed_u32::bit_and(a3282, a3283)
    };
    let a3285 = 0u32;
    if equal(a3284, a3285) {
        let a3288 = {
            let a3286 = v0;
            let a3287 = 8u32;
            fixed_u32::bit_and(a3286, a3287)
        };
        let a3289 = 0u32;
        if equal(a3288, a3289) {
            let a3292 = {
                let a3290 = v0;
                let a3291 = 4u32;
                fixed_u32::bit_and(a3290, a3291)
            };
            let a3293 = 0u32;
            if equal(a3292, a3293) {
                let a3296 = {
                    let a3294 = v0;
                    let a3295 = 2u32;
                    fixed_u32::bit_and(a3294, a3295)
                };
                let a3297 = 0u32;
                if equal(a3296, a3297) {
                    let a3300 = {
                        let a3298 = v0;
                        let a3299 = 1u32;
                        fixed_u32::bit_and(a3298, a3299)
                    };
                    let a3301 = 0u32;
                    if equal(a3300, a3301) {
                        Bytes::lit(&[0x00])
                    } else {
                        Bytes::lit(&[0x01])
                    }
                } else {
                    let a3304 = {
                        let a3302 = v0;
                        let a3303 = 1u32;
                        fixed_u32::bit_and(a3302, a3303)
                    };
                    let a3305 = 0u32;
                    if equal(a3304, a3305) {
                        Bytes::lit(&[0x02])
                    } else {
                        Bytes::lit(&[0x03])
                    }
                }
            } else {
                let a3308 = {
                    let a3306 = v0;
                    let a3307 = 2u32;
                    fixed_u32::bit_and(a3306, a3307)
                };
                let a3309 = 0u32;
                if equal(a3308, a3309) {
                    let a3312 = {
                        let a3310 = v0;
                        let a3311 = 1u32;
                        fixed_u32::bit_and(a3310, a3311)
                    };
                    let a3313 = 0u32;
                    if equal(a3312, a3313) {
                        Bytes::lit(&[0x04])
                    } else {
                        Bytes::lit(&[0x05])
                    }
                } else {
                    let a3316 = {
                        let a3314 = v0;
                        let a3315 = 1u32;
                        fixed_u32::bit_and(a3314, a3315)
                    };
                    let a3317 = 0u32;
                    if equal(a3316, a3317) {
                        Bytes::lit(&[0x06])
                    } else {
                        Bytes::lit(&[0x07])
                    }
                }
            }
        } else {
            let a3320 = {
                let a3318 = v0;
                let a3319 = 4u32;
                fixed_u32::bit_and(a3318, a3319)
            };
            let a3321 = 0u32;
            if equal(a3320, a3321) {
                let a3324 = {
                    let a3322 = v0;
                    let a3323 = 2u32;
                    fixed_u32::bit_and(a3322, a3323)
                };
                let a3325 = 0u32;
                if equal(a3324, a3325) {
                    let a3328 = {
                        let a3326 = v0;
                        let a3327 = 1u32;
                        fixed_u32::bit_and(a3326, a3327)
                    };
                    let a3329 = 0u32;
                    if equal(a3328, a3329) {
                        Bytes::lit(&[0x08])
                    } else {
                        Bytes::lit(&[0x09])
                    }
                } else {
                    let a3332 = {
                        let a3330 = v0;
                        let a3331 = 1u32;
                        fixed_u32::bit_and(a3330, a3331)
                    };
                    let a3333 = 0u32;
                    if equal(a3332, a3333) {
                        Bytes::lit(&[0x0a])
                    } else {
                        Bytes::lit(&[0x0b])
                    }
                }
            } else {
                let a3336 = {
                    let a3334 = v0;
                    let a3335 = 2u32;
                    fixed_u32::bit_and(a3334, a3335)
                };
                let a3337 = 0u32;
                if equal(a3336, a3337) {
                    let a3340 = {
                        let a3338 = v0;
                        let a3339 = 1u32;
                        fixed_u32::bit_and(a3338, a3339)
                    };
                    let a3341 = 0u32;
                    if equal(a3340, a3341) {
                        Bytes::lit(&[0x0c])
                    } else {
                        Bytes::lit(&[0x0d])
                    }
                } else {
                    let a3344 = {
                        let a3342 = v0;
                        let a3343 = 1u32;
                        fixed_u32::bit_and(a3342, a3343)
                    };
                    let a3345 = 0u32;
                    if equal(a3344, a3345) {
                        Bytes::lit(&[0x0e])
                    } else {
                        Bytes::lit(&[0x0f])
                    }
                }
            }
        }
    } else {
        let a3348 = {
            let a3346 = v0;
            let a3347 = 8u32;
            fixed_u32::bit_and(a3346, a3347)
        };
        let a3349 = 0u32;
        if equal(a3348, a3349) {
            let a3352 = {
                let a3350 = v0;
                let a3351 = 4u32;
                fixed_u32::bit_and(a3350, a3351)
            };
            let a3353 = 0u32;
            if equal(a3352, a3353) {
                let a3356 = {
                    let a3354 = v0;
                    let a3355 = 2u32;
                    fixed_u32::bit_and(a3354, a3355)
                };
                let a3357 = 0u32;
                if equal(a3356, a3357) {
                    let a3360 = {
                        let a3358 = v0;
                        let a3359 = 1u32;
                        fixed_u32::bit_and(a3358, a3359)
                    };
                    let a3361 = 0u32;
                    if equal(a3360, a3361) {
                        Bytes::lit(&[0x10])
                    } else {
                        Bytes::lit(&[0x11])
                    }
                } else {
                    let a3364 = {
                        let a3362 = v0;
                        let a3363 = 1u32;
                        fixed_u32::bit_and(a3362, a3363)
                    };
                    let a3365 = 0u32;
                    if equal(a3364, a3365) {
                        Bytes::lit(&[0x12])
                    } else {
                        Bytes::lit(&[0x13])
                    }
                }
            } else {
                let a3368 = {
                    let a3366 = v0;
                    let a3367 = 2u32;
                    fixed_u32::bit_and(a3366, a3367)
                };
                let a3369 = 0u32;
                if equal(a3368, a3369) {
                    let a3372 = {
                        let a3370 = v0;
                        let a3371 = 1u32;
                        fixed_u32::bit_and(a3370, a3371)
                    };
                    let a3373 = 0u32;
                    if equal(a3372, a3373) {
                        Bytes::lit(&[0x14])
                    } else {
                        Bytes::lit(&[0x15])
                    }
                } else {
                    let a3376 = {
                        let a3374 = v0;
                        let a3375 = 1u32;
                        fixed_u32::bit_and(a3374, a3375)
                    };
                    let a3377 = 0u32;
                    if equal(a3376, a3377) {
                        Bytes::lit(&[0x16])
                    } else {
                        Bytes::lit(&[0x17])
                    }
                }
            }
        } else {
            let a3380 = {
                let a3378 = v0;
                let a3379 = 4u32;
                fixed_u32::bit_and(a3378, a3379)
            };
            let a3381 = 0u32;
            if equal(a3380, a3381) {
                let a3384 = {
                    let a3382 = v0;
                    let a3383 = 2u32;
                    fixed_u32::bit_and(a3382, a3383)
                };
                let a3385 = 0u32;
                if equal(a3384, a3385) {
                    let a3388 = {
                        let a3386 = v0;
                        let a3387 = 1u32;
                        fixed_u32::bit_and(a3386, a3387)
                    };
                    let a3389 = 0u32;
                    if equal(a3388, a3389) {
                        Bytes::lit(&[0x18])
                    } else {
                        Bytes::lit(&[0x19])
                    }
                } else {
                    let a3392 = {
                        let a3390 = v0;
                        let a3391 = 1u32;
                        fixed_u32::bit_and(a3390, a3391)
                    };
                    let a3393 = 0u32;
                    if equal(a3392, a3393) {
                        Bytes::lit(&[0x1a])
                    } else {
                        Bytes::lit(&[0x1b])
                    }
                }
            } else {
                let a3396 = {
                    let a3394 = v0;
                    let a3395 = 2u32;
                    fixed_u32::bit_and(a3394, a3395)
                };
                let a3397 = 0u32;
                if equal(a3396, a3397) {
                    let a3400 = {
                        let a3398 = v0;
                        let a3399 = 1u32;
                        fixed_u32::bit_and(a3398, a3399)
                    };
                    let a3401 = 0u32;
                    if equal(a3400, a3401) {
                        Bytes::lit(&[0x1c])
                    } else {
                        Bytes::lit(&[0x1d])
                    }
                } else {
                    let a3404 = {
                        let a3402 = v0;
                        let a3403 = 1u32;
                        fixed_u32::bit_and(a3402, a3403)
                    };
                    let a3405 = 0u32;
                    if equal(a3404, a3405) {
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
        let a3408 = {
            let a3406 = v0;
            let a3407 = v1;
            fixed_u32::bit_xor(a3406, a3407)
        };
        let a3409 = v2;
        fixed_u32::bit_xor(a3408, a3409)
    };
    let v4: u32 = {
        let a3418 = {
            let a3414 = {
                let a3410 = v0;
                let a3411 = v1;
                fixed_u32::bit_and(a3410, a3411)
            };
            let a3415 = {
                let a3412 = v0;
                let a3413 = v2;
                fixed_u32::bit_and(a3412, a3413)
            };
            fixed_u32::bit_or(a3414, a3415)
        };
        let a3419 = {
            let a3416 = v1;
            let a3417 = v2;
            fixed_u32::bit_and(a3416, a3417)
        };
        fixed_u32::bit_or(a3418, a3419)
    };
    let a3426 = v0;
    let a3427 = {
        let a3424 = f105(v1, 1u32);
        let a3425 = {
            let a3422 = f105(v2, 2u32);
            let a3423 = {
                let a3420 = f105(v3, 3u32);
                let a3421 = f105(v4, 4u32);
                fixed_u32::bit_or(a3420, a3421)
            };
            fixed_u32::bit_or(a3422, a3423)
        };
        fixed_u32::bit_or(a3424, a3425)
    };
    fixed_u32::bit_or(a3426, a3427)
}

pub fn f162(v0: u32, v1: u32) -> u32 {
    let a3432 = v0;
    let a3433 = {
        let a3430 = f105(v1, 1u32);
        let a3431 = f105({
            let a3428 = v0;
            let a3429 = v1;
            fixed_u32::bit_xor(a3428, a3429)
        }, 2u32);
        fixed_u32::bit_or(a3430, a3431)
    };
    fixed_u32::bit_or(a3432, a3433)
}

pub fn f163(v0: u32, v1: u32) -> u32 {
    let m3436 = {
        let a3434 = v0;
        let a3435 = v1;
        fixed_u32::shift_right(a3434, a3435)
    };
    match m3436 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn run(p0: Adt0, p1: List<Bytes>, p2: u64) -> R<Result<Bytes, Adt1>> {
    f0(p0, p1, p2)
}
