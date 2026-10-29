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
    F4(Bytes),
}

impl Fn0 {
    pub fn apply(&self, p0: (u64, Bytes)) -> R<(u64, Bytes)> {
        match self {
            Fn0::F4(k0) => f4(k0.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn1 {
    F12,
}

impl Fn1 {
    pub fn apply(&self, p0: List<Bytes>, p1: Adt2) -> List<Bytes> {
        match self {
            Fn1::F12 => f12(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn2 {
    F15,
    F62,
}

impl Fn2 {
    pub fn apply(&self, p0: List<Bytes>, p1: Bytes) -> List<Bytes> {
        match self {
            Fn2::F15 => f15(p0, p1),
            Fn2::F62 => f62(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn3 {
    F18(u32, Adt1),
}

impl Fn3 {
    pub fn apply(&self, p0: (Adt7, (List<Adt2>, u64)), p1: Adt5) -> R<(Adt7, (List<Adt2>, u64))> {
        match self {
            Fn3::F18(k0, k1) => f18(*k0, k1.clone(), p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn4 {
    F20((Adt7, (List<Adt2>, u64))),
}

impl Fn4 {
    pub fn apply(&self, p0: (u64, List<Adt2>)) -> R<(u64, List<Adt2>)> {
        match self {
            Fn4::F20(k0) => f20(k0.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn5 {
    F24(u32, Bytes, Adt1, Adt6),
}

impl Fn5 {
    pub fn apply(&self, p0: (u64, (List<Adt5>, List<Adt2>))) -> R<(u64, (List<Adt5>, List<Adt2>))> {
        match self {
            Fn5::F24(k0, k1, k2, k3) => f24(*k0, k1.clone(), k2.clone(), k3.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn6 {
    F30,
}

impl Fn6 {
    pub fn apply(&self, p0: List<Bytes>) -> Option<List<Bytes>> {
        match self {
            Fn6::F30 => f30(p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn7 {
    F33(u32, u64, u64, Adt1, u64, Bytes),
}

impl Fn7 {
    pub fn apply(&self, p0: (u64, (Adt1, List<Adt2>))) -> R<(u64, (Adt1, List<Adt2>))> {
        match self {
            Fn7::F33(k0, k1, k2, k3, k4, k5) => f33(*k0, *k1, *k2, k3.clone(), *k4, k5.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn8 {
    F45,
}

impl Fn8 {
    pub fn apply(&self, p0: List<Adt2>, p1: Adt2) -> List<Adt2> {
        match self {
            Fn8::F45 => f45(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn9 {
    F58(u32, Adt1),
}

impl Fn9 {
    pub fn apply(&self, p0: Adt8) -> R<Option<Adt8>> {
        match self {
            Fn9::F58(k0, k1) => f58(*k0, k1.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn10 {
    F61,
}

impl Fn10 {
    pub fn apply(&self, p0: (Option<Bytes>, List<Bytes>), p1: Bytes) -> (Option<Bytes>, List<Bytes>) {
        match self {
            Fn10::F61 => f61(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn11 {
    F65,
}

impl Fn11 {
    pub fn apply(&self, p0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
        match self {
            Fn11::F65 => f65(p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn12 {
    F72,
}

impl Fn12 {
    pub fn apply(&self, p0: Adt4, p1: Adt10) -> Adt4 {
        match self {
            Fn12::F72 => f72(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn13 {
    F74(Adt1, Adt4),
}

impl Fn13 {
    pub fn apply(&self, p0: (u64, Adt12)) -> R<(u64, Adt12)> {
        match self {
            Fn13::F74(k0, k1) => f74(k0.clone(), k1.clone(), p0),
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
    Ok(f5(f6(f7(f8(f9(), 0u32), v0)?, v1)?))
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
    f10(v0, v1)
}

pub fn f5(v0: Adt0) -> Bytes {
    let v1: List<Bytes> = f11(Fn1::F12, List::<Bytes>::nil(), match v0.clone() {
        Adt0::C0(_, _, _, _, _, _, h8) => {
            h8
        }
    });
    let v2: Bytes = f13(f14(Fn2::F15, List::<Bytes>::nil(), v1));
    let m13 = {
        let a10 = v2.clone();
        let a11 = 0u64;
        let a12 = match v0 {
            Adt0::C0(_, _, h9, _, _, _, _) => {
                h9
            }
        };
        slice_bytes(a10, a11, a12)
    };
    match m13 {
        None => {
            v2
        }
        Some(v3) => {
            v3
        }
    }
}

pub fn f6(v0: Adt6, v1: u64) -> R<Adt0> {
    let v2: Adt1 = match v0.clone() {
        Adt6::C0(h14, _, _, _, _, _, _) => {
            h14
        }
    };
    let v3: u32 = match v0.clone() {
        Adt6::C0(_, h15, _, _, _, _, _) => {
            h15
        }
    };
    let v4: Bytes = match v0.clone() {
        Adt6::C0(_, _, _, h16, _, _, _) => {
            h16
        }
    };
    let v5: (Adt7, List<Adt2>) = f16(v2.clone(), v3, v4.clone(), 0u64, {
        let a17 = v4.clone();
        length_bytes(a17)
    }, match v0.clone() {
        Adt6::C0(_, _, h18, _, _, _, _) => {
            h18
        }
    }, match v0.clone() {
        Adt6::C0(_, _, _, _, _, h19, _) => {
            h19
        }
    })?;
    let v6: (Adt7, (List<Adt2>, u64)) = f17(Fn3::F18(v3, v2.clone()), ({
        let (h20, _) = v5;
        h20
    }, ({
        let (_, h21) = v5;
        h21
    }, {
        let a23 = match v0.clone() {
            Adt6::C0(_, _, _, _, h22, _, _) => {
                h22
            }
        };
        length_list(a23)
    })), match v0.clone() {
        Adt6::C0(_, _, _, _, h24, _, _) => {
            h24
        }
    })?;
    let v7: (u64, List<Adt2>) = f19(Fn4::F20(v6.clone()), {
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
    Ok(Adt0::C0(f21(v3), {
        let a36 = {
            let a33 = match v0.clone() {
                Adt6::C0(_, _, h32, _, _, _, _) => {
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
        Adt6::C0(_, _, _, _, _, _, h38) => {
            h38
        }
    }, f22({
        let (_, h39) = v7;
        h39
    })))
}

pub fn f7(v0: Adt6, v1: Bytes) -> R<Adt6> {
    let v2: Bytes = {
        let a41 = match v0.clone() {
            Adt6::C0(_, _, _, h40, _, _, _) => {
                h40
            }
        };
        let a42 = v1;
        append_bytes(a41, a42)
    };
    let v3: u64 = {
        let a43 = v2.clone();
        length_bytes(a43)
    };
    let v4: u64 = {
        let a44 = v3;
        let a45 = 0u64;
        if nat_eq(a44, a45) {
            0u64
        } else {
            let a48 = {
                let a46 = v3;
                let a47 = 1u64;
                nat_sub(a46, a47)
            };
            let a49 = 1024u64;
            let a50 = 0u64;
            nat_quot(a48, a49, a50)
        }
    };
    let v5: u32 = match v0.clone() {
        Adt6::C0(_, h51, _, _, _, _, _) => {
            h51
        }
    };
    let v6: Adt1 = match v0.clone() {
        Adt6::C0(h52, _, _, _, _, _, _) => {
            h52
        }
    };
    let v7: (u64, (List<Adt5>, List<Adt2>)) = f23(Fn5::F24(v5, v2.clone(), v6.clone(), v0.clone()), v4, (0u64, (match v0.clone() {
        Adt6::C0(_, _, _, _, h53, _, _) => {
            h53
        }
    }, match v0.clone() {
        Adt6::C0(_, _, _, _, _, h54, _) => {
            h54
        }
    })))?;
    let v8: u64 = {
        let a55 = v4;
        let a56 = 1024u64;
        nat_mul(a55, a56)?
    };
    Ok(Adt6::C0(v6, v5, {
        let a58 = match v0.clone() {
            Adt6::C0(_, _, h57, _, _, _, _) => {
                h57
            }
        };
        let a59 = v4;
        nat_add(a58, a59)?
    }, {
        let m65 = {
            let a62 = v2;
            let a63 = v8;
            let a64 = {
                let a60 = v3;
                let a61 = v8;
                nat_sub(a60, a61)
            };
            slice_bytes(a62, a63, a64)
        };
        match m65 {
            None => {
                Bytes::lit(&[])
            }
            Some(v9) => {
                v9
            }
        }
    }, {
        let (h66, _) = {
            let (_, h67) = v7.clone();
            h67
        };
        h66
    }, {
        let (_, h68) = {
            let (_, h69) = v7;
            h69
        };
        h68
    }, match v0 {
        Adt6::C0(_, _, _, _, _, _, h70) => {
            h70
        }
    }))
}

pub fn f8(v0: Adt1, v1: u32) -> Adt6 {
    Adt6::C0(v0, v1, 0u64, Bytes::lit(&[]), List::<Adt5>::nil(), List::<Adt2>::nil(), List::<Adt2>::nil())
}

pub fn f9() -> Adt1 {
    Adt1::C0(1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, 1359893119u32, 2600822924u32, 528734635u32, 1541459225u32)
}

pub fn f10(v0: Bytes, v1: (u64, Bytes)) -> R<(u64, Bytes)> {
    let v2: u64 = {
        let (h71, _) = v1;
        h71
    };
    let v3: u32 = f25(v0.clone(), {
        let a72 = v0;
        length_bytes(a72)
    }, v2);
    Ok(({
        let a73 = v2;
        let a74 = 1u64;
        nat_add(a73, a74)?
    }, {
        let a80 = {
            let (_, h75) = v1;
            h75
        };
        let a81 = {
            let a78 = f26(f27(v3, 4u32));
            let a79 = f26({
                let a76 = v3;
                let a77 = 15u32;
                fixed_u32::bit_and(a76, a77)
            });
            append_bytes(a78, a79)
        };
        append_bytes(a80, a81)
    }))
}

pub fn f11(v0: Fn1, p85: List<Bytes>, p86: List<Adt2>) -> List<Bytes> {
    let m84 = (p85, p86);
    {
        let mut m84 = m84;
        loop {
            let step = {
                let (v1, v2) = m84;
                let m82 = v2;
                match m82.uncons() {
                    None => {
                        Err::<(List<Bytes>, List<Adt2>), List<Bytes>>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(List<Bytes>, List<Adt2>), List<Bytes>>(({
                            let c83: Fn1 = v0.clone();
                            c83.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m84 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f12(v0: List<Bytes>, v1: Adt2) -> List<Bytes> {
    let m87 = v1;
    match m87 {
        Adt2::C0(v2) => {
            let a89 = match v2.clone() {
                Adt3::C0(h88, _, _, _, _, _, _, _, _, _, _, _) => {
                    h88
                }
            };
            let a90 = 2u64;
            if nat_eq(a89, a90) {
                List::cons(f28(match v2 {
                    Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h91) => {
                        h91
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

pub fn f13(v0: List<Bytes>) -> Bytes {
    let m93 = {
        let (h92, _) = f29(Fn6::F30, 64u64, v0);
        h92
    };
    match m93.uncons() {
        None => {
            Bytes::lit(&[])
        }
        Some((v1, _)) => {
            v1
        }
    }
}

pub fn f14(v0: Fn2, p97: List<Bytes>, p98: List<Bytes>) -> List<Bytes> {
    let m96 = (p97, p98);
    {
        let mut m96 = m96;
        loop {
            let step = {
                let (v1, v2) = m96;
                let m94 = v2;
                match m94.uncons() {
                    None => {
                        Err::<(List<Bytes>, List<Bytes>), List<Bytes>>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(List<Bytes>, List<Bytes>), List<Bytes>>(({
                            let c95: Fn2 = v0.clone();
                            c95.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m96 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f15(v0: List<Bytes>, v1: Bytes) -> List<Bytes> {
    List::cons(v1, v0)
}

pub fn f16(v0: Adt1, v1: u32, v2: Bytes, v3: u64, v4: u64, v5: u64, v6: List<Adt2>) -> R<(Adt7, List<Adt2>)> {
    let v7: u64 = {
        let a99 = v3;
        let a100 = v4;
        nat_add(a99, a100)?
    };
    let v8: u64 = f31(v4)?;
    let v9: (u64, (Adt1, List<Adt2>)) = f32(Fn7::F33(v1, v3, v5, v0.clone(), v7, v2.clone()), {
        let a101 = v8;
        let a102 = 64u64;
        let a103 = 0u64;
        nat_quot(a101, a102, a103)
    }, (0u64, (v0.clone(), v6)))?;
    Ok((f34(v0, v1, v2, v7, v3, v5, v8, {
        let (h104, _) = {
            let (_, h105) = v9.clone();
            h105
        };
        h104
    }, {
        let a106 = v4;
        let a107 = v8;
        nat_sub(a106, a107)
    }, {
        let a108 = f35();
        let a109 = f36(v8);
        fixed_u32::bit_or(a108, a109)
    })?, {
        let (_, h110) = {
            let (_, h111) = v9;
            h111
        };
        h110
    }))
}

pub fn f17(v0: Fn3, p115: (Adt7, (List<Adt2>, u64)), p116: List<Adt5>) -> R<(Adt7, (List<Adt2>, u64))> {
    let m114 = (p115, p116);
    {
        let mut m114 = m114;
        loop {
            let step = {
                let (v1, v2) = m114;
                let m112 = v2;
                match m112.uncons() {
                    None => {
                        Err::<((Adt7, (List<Adt2>, u64)), List<Adt5>), R<(Adt7, (List<Adt2>, u64))>>(Ok(v1))
                    }
                    Some((v3, v4)) => {
                        Ok::<((Adt7, (List<Adt2>, u64)), List<Adt5>), R<(Adt7, (List<Adt2>, u64))>>(({
                            let c113: Fn3 = v0.clone();
                            c113.apply(v1, v3)?
                        }, v4))
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

pub fn f18(v0: u32, v1: Adt1, v2: (Adt7, (List<Adt2>, u64)), v3: Adt5) -> R<(Adt7, (List<Adt2>, u64))> {
    let v4: Adt3 = f37({
        let (h117, _) = v2.clone();
        h117
    })?;
    Ok((f38(v3.clone(), Adt5::C0(f39(match v4.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h118) => {
            h118
        }
    }), {
        let m121 = {
            let (h120, _) = v2.clone();
            h120
        };
        match m121 {
            Adt7::C0(_, h119, _, _, _, _, _, _, _, _) => {
                h119
            }
        }
    }, {
        let m124 = {
            let (h123, _) = v2.clone();
            h123
        };
        match m124 {
            Adt7::C0(_, _, h122, _, _, _, _, _, _, _) => {
                h122
            }
        }
    }), v1, v0)?, (List::cons(Adt2::C2(v3, {
        let (_, h125) = {
            let (_, h126) = v2.clone();
            h126
        };
        h125
    }), List::cons(Adt2::C0(v4), {
        let (h127, _) = {
            let (_, h128) = v2.clone();
            h128
        };
        h127
    })), {
        let a131 = {
            let (_, h129) = {
                let (_, h130) = v2;
                h130
            };
            h129
        };
        let a132 = 1u64;
        nat_sub(a131, a132)
    })))
}

pub fn f19(v0: Fn4, p136: u64, p137: (u64, List<Adt2>)) -> R<(u64, List<Adt2>)> {
    let m135 = (p136, p137);
    {
        let mut m135 = m135;
        loop {
            let step = {
                let (v1, v2) = m135;
                let m133 = v1;
                if m133 == 0 {
                    Err::<(u64, (u64, List<Adt2>)), R<(u64, List<Adt2>)>>(Ok(v2))
                } else {
                    let v3 = m133 - 1;
                    Ok::<(u64, (u64, List<Adt2>)), R<(u64, List<Adt2>)>>((v3, {
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

pub fn f20(v0: (Adt7, (List<Adt2>, u64)), v1: (u64, List<Adt2>)) -> R<(u64, List<Adt2>)> {
    Ok(({
        let a139 = {
            let (h138, _) = v1.clone();
            h138
        };
        let a140 = 1u64;
        nat_add(a139, a140)?
    }, List::cons(Adt2::C0(f37(f40({
        let (h141, _) = v0;
        h141
    }, {
        let (h142, _) = v1;
        h142
    }))?), {
        let (_, h143) = v1;
        h143
    })))
}

pub fn f21(v0: u32) -> u64 {
    let a144 = v0;
    let a145 = f41();
    if equal(a144, a145) {
        1u64
    } else {
        let a146 = v0;
        let a147 = f42();
        if equal(a146, a147) {
            2u64
        } else {
            let a148 = v0;
            let a149 = f43();
            if equal(a148, a149) {
                3u64
            } else {
                0u64
            }
        }
    }
}

pub fn f22(v0: List<Adt2>) -> List<Adt2> {
    f44(Fn8::F45, List::<Adt2>::nil(), v0)
}

pub fn f23(v0: Fn5, p153: u64, p154: (u64, (List<Adt5>, List<Adt2>))) -> R<(u64, (List<Adt5>, List<Adt2>))> {
    let m152 = (p153, p154);
    {
        let mut m152 = m152;
        loop {
            let step = {
                let (v1, v2) = m152;
                let m150 = v1;
                if m150 == 0 {
                    Err::<(u64, (u64, (List<Adt5>, List<Adt2>))), R<(u64, (List<Adt5>, List<Adt2>))>>(Ok(v2))
                } else {
                    let v3 = m150 - 1;
                    Ok::<(u64, (u64, (List<Adt5>, List<Adt2>))), R<(u64, (List<Adt5>, List<Adt2>))>>((v3, {
                        let c151: Fn5 = v0.clone();
                        c151.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m152 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f24(v0: u32, v1: Bytes, v2: Adt1, v3: Adt6, v4: (u64, (List<Adt5>, List<Adt2>))) -> R<(u64, (List<Adt5>, List<Adt2>))> {
    let v5: u64 = {
        let a157 = match v3 {
            Adt6::C0(_, _, h155, _, _, _, _) => {
                h155
            }
        };
        let a158 = {
            let (h156, _) = v4.clone();
            h156
        };
        nat_add(a157, a158)?
    };
    let v6: (Adt7, List<Adt2>) = f16(v2.clone(), v0, v1, {
        let a160 = {
            let (h159, _) = v4.clone();
            h159
        };
        let a161 = 1024u64;
        nat_mul(a160, a161)?
    }, 1024u64, v5, {
        let (_, h162) = {
            let (_, h163) = v4.clone();
            h163
        };
        h162
    })?;
    let v7: Adt3 = f37({
        let (h164, _) = v6;
        h164
    })?;
    Ok(({
        let a166 = {
            let (h165, _) = v4;
            h165
        };
        let a167 = 1u64;
        nat_add(a166, a167)?
    }, f46(v2, v0, Adt8::C0(Adt5::C0(f39(match v7.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h168) => {
            h168
        }
    }), v5, 1u64), {
        let a169 = v5;
        let a170 = 1u64;
        nat_add(a169, a170)?
    }, {
        let (h171, _) = {
            let (_, h172) = v4;
            h172
        };
        h171
    }, List::cons(Adt2::C0(v7), {
        let (_, h173) = v6;
        h173
    })))?))
}

pub fn f25(v0: Bytes, v1: u64, v2: u64) -> u32 {
    let a174 = v2;
    let a175 = v1;
    if nat_lt(a174, a175) {
        let m178 = {
            let a176 = v0;
            let a177 = v2;
            index_bytes(a176, a177)
        };
        match m178 {
            None => {
                0u32
            }
            Some(v3) => {
                let m180 = {
                    let a179 = v3;
                    fixed_u32::convert(i128::from(a179))
                };
                match m180 {
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

pub fn f26(v0: u32) -> Bytes {
    let a183 = {
        let a181 = v0;
        let a182 = 8u32;
        fixed_u32::bit_and(a181, a182)
    };
    let a184 = 0u32;
    if equal(a183, a184) {
        let a187 = {
            let a185 = v0;
            let a186 = 4u32;
            fixed_u32::bit_and(a185, a186)
        };
        let a188 = 0u32;
        if equal(a187, a188) {
            let a191 = {
                let a189 = v0;
                let a190 = 2u32;
                fixed_u32::bit_and(a189, a190)
            };
            let a192 = 0u32;
            if equal(a191, a192) {
                let a195 = {
                    let a193 = v0;
                    let a194 = 1u32;
                    fixed_u32::bit_and(a193, a194)
                };
                let a196 = 0u32;
                if equal(a195, a196) {
                    Bytes::lit(&[0x30])
                } else {
                    Bytes::lit(&[0x31])
                }
            } else {
                let a199 = {
                    let a197 = v0;
                    let a198 = 1u32;
                    fixed_u32::bit_and(a197, a198)
                };
                let a200 = 0u32;
                if equal(a199, a200) {
                    Bytes::lit(&[0x32])
                } else {
                    Bytes::lit(&[0x33])
                }
            }
        } else {
            let a203 = {
                let a201 = v0;
                let a202 = 2u32;
                fixed_u32::bit_and(a201, a202)
            };
            let a204 = 0u32;
            if equal(a203, a204) {
                let a207 = {
                    let a205 = v0;
                    let a206 = 1u32;
                    fixed_u32::bit_and(a205, a206)
                };
                let a208 = 0u32;
                if equal(a207, a208) {
                    Bytes::lit(&[0x34])
                } else {
                    Bytes::lit(&[0x35])
                }
            } else {
                let a211 = {
                    let a209 = v0;
                    let a210 = 1u32;
                    fixed_u32::bit_and(a209, a210)
                };
                let a212 = 0u32;
                if equal(a211, a212) {
                    Bytes::lit(&[0x36])
                } else {
                    Bytes::lit(&[0x37])
                }
            }
        }
    } else {
        let a215 = {
            let a213 = v0;
            let a214 = 4u32;
            fixed_u32::bit_and(a213, a214)
        };
        let a216 = 0u32;
        if equal(a215, a216) {
            let a219 = {
                let a217 = v0;
                let a218 = 2u32;
                fixed_u32::bit_and(a217, a218)
            };
            let a220 = 0u32;
            if equal(a219, a220) {
                let a223 = {
                    let a221 = v0;
                    let a222 = 1u32;
                    fixed_u32::bit_and(a221, a222)
                };
                let a224 = 0u32;
                if equal(a223, a224) {
                    Bytes::lit(&[0x38])
                } else {
                    Bytes::lit(&[0x39])
                }
            } else {
                let a227 = {
                    let a225 = v0;
                    let a226 = 1u32;
                    fixed_u32::bit_and(a225, a226)
                };
                let a228 = 0u32;
                if equal(a227, a228) {
                    Bytes::lit(&[0x61])
                } else {
                    Bytes::lit(&[0x62])
                }
            }
        } else {
            let a231 = {
                let a229 = v0;
                let a230 = 2u32;
                fixed_u32::bit_and(a229, a230)
            };
            let a232 = 0u32;
            if equal(a231, a232) {
                let a235 = {
                    let a233 = v0;
                    let a234 = 1u32;
                    fixed_u32::bit_and(a233, a234)
                };
                let a236 = 0u32;
                if equal(a235, a236) {
                    Bytes::lit(&[0x63])
                } else {
                    Bytes::lit(&[0x64])
                }
            } else {
                let a239 = {
                    let a237 = v0;
                    let a238 = 1u32;
                    fixed_u32::bit_and(a237, a238)
                };
                let a240 = 0u32;
                if equal(a239, a240) {
                    Bytes::lit(&[0x65])
                } else {
                    Bytes::lit(&[0x66])
                }
            }
        }
    }
}

pub fn f27(v0: u32, v1: u32) -> u32 {
    let m243 = {
        let a241 = v0;
        let a242 = v1;
        fixed_u32::shift_right(a241, a242)
    };
    match m243 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f28(v0: Adt4) -> Bytes {
    let a288 = f47(match v0.clone() {
        Adt4::C0(h244, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h244
        }
    });
    let a289 = {
        let a286 = f47(match v0.clone() {
            Adt4::C0(_, h245, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                h245
            }
        });
        let a287 = {
            let a284 = f47(match v0.clone() {
                Adt4::C0(_, _, h246, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                    h246
                }
            });
            let a285 = {
                let a282 = f47(match v0.clone() {
                    Adt4::C0(_, _, _, h247, _, _, _, _, _, _, _, _, _, _, _, _) => {
                        h247
                    }
                });
                let a283 = {
                    let a280 = f47(match v0.clone() {
                        Adt4::C0(_, _, _, _, h248, _, _, _, _, _, _, _, _, _, _, _) => {
                            h248
                        }
                    });
                    let a281 = {
                        let a278 = f47(match v0.clone() {
                            Adt4::C0(_, _, _, _, _, h249, _, _, _, _, _, _, _, _, _, _) => {
                                h249
                            }
                        });
                        let a279 = {
                            let a276 = f47(match v0.clone() {
                                Adt4::C0(_, _, _, _, _, _, h250, _, _, _, _, _, _, _, _, _) => {
                                    h250
                                }
                            });
                            let a277 = {
                                let a274 = f47(match v0.clone() {
                                    Adt4::C0(_, _, _, _, _, _, _, h251, _, _, _, _, _, _, _, _) => {
                                        h251
                                    }
                                });
                                let a275 = {
                                    let a272 = f47(match v0.clone() {
                                        Adt4::C0(_, _, _, _, _, _, _, _, h252, _, _, _, _, _, _, _) => {
                                            h252
                                        }
                                    });
                                    let a273 = {
                                        let a270 = f47(match v0.clone() {
                                            Adt4::C0(_, _, _, _, _, _, _, _, _, h253, _, _, _, _, _, _) => {
                                                h253
                                            }
                                        });
                                        let a271 = {
                                            let a268 = f47(match v0.clone() {
                                                Adt4::C0(_, _, _, _, _, _, _, _, _, _, h254, _, _, _, _, _) => {
                                                    h254
                                                }
                                            });
                                            let a269 = {
                                                let a266 = f47(match v0.clone() {
                                                    Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h255, _, _, _, _) => {
                                                        h255
                                                    }
                                                });
                                                let a267 = {
                                                    let a264 = f47(match v0.clone() {
                                                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h256, _, _, _) => {
                                                            h256
                                                        }
                                                    });
                                                    let a265 = {
                                                        let a262 = f47(match v0.clone() {
                                                            Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h257, _, _) => {
                                                                h257
                                                            }
                                                        });
                                                        let a263 = {
                                                            let a260 = f47(match v0.clone() {
                                                                Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h258, _) => {
                                                                    h258
                                                                }
                                                            });
                                                            let a261 = f47(match v0 {
                                                                Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h259) => {
                                                                    h259
                                                                }
                                                            });
                                                            append_bytes(a260, a261)
                                                        };
                                                        append_bytes(a262, a263)
                                                    };
                                                    append_bytes(a264, a265)
                                                };
                                                append_bytes(a266, a267)
                                            };
                                            append_bytes(a268, a269)
                                        };
                                        append_bytes(a270, a271)
                                    };
                                    append_bytes(a272, a273)
                                };
                                append_bytes(a274, a275)
                            };
                            append_bytes(a276, a277)
                        };
                        append_bytes(a278, a279)
                    };
                    append_bytes(a280, a281)
                };
                append_bytes(a282, a283)
            };
            append_bytes(a284, a285)
        };
        append_bytes(a286, a287)
    };
    append_bytes(a288, a289)
}

pub fn f29(v0: Fn6, p294: u64, p295: List<Bytes>) -> (List<Bytes>, bool) {
    let m293 = (p294, p295);
    {
        let mut m293 = m293;
        loop {
            let step = {
                let (v1, v2) = m293;
                let m290 = v1;
                if m290 == 0 {
                    Err::<(u64, List<Bytes>), (List<Bytes>, bool)>((v2, false))
                } else {
                    let v3 = m290 - 1;
                    let m292 = {
                        let c291: Fn6 = v0.clone();
                        c291.apply(v2.clone())
                    };
                    match m292 {
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
                Ok(next) => { m293 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f30(v0: List<Bytes>) -> Option<List<Bytes>> {
    let a297 = {
        let a296 = v0.clone();
        length_list(a296)
    };
    let a298 = 1u64;
    if nat_le(a297, a298) {
        None::<List<Bytes>>
    } else {
        Some(f48(v0))
    }
}

pub fn f31(v0: u64) -> R<u64> {
    let a299 = v0;
    let a300 = 0u64;
    if nat_eq(a299, a300) {
        Ok(0u64)
    } else {
        let a306 = {
            let a303 = {
                let a301 = v0;
                let a302 = 1u64;
                nat_sub(a301, a302)
            };
            let a304 = 64u64;
            let a305 = 0u64;
            nat_quot(a303, a304, a305)
        };
        let a307 = 64u64;
        nat_mul(a306, a307)
    }
}

pub fn f32(v0: Fn7, p311: u64, p312: (u64, (Adt1, List<Adt2>))) -> R<(u64, (Adt1, List<Adt2>))> {
    let m310 = (p311, p312);
    {
        let mut m310 = m310;
        loop {
            let step = {
                let (v1, v2) = m310;
                let m308 = v1;
                if m308 == 0 {
                    Err::<(u64, (u64, (Adt1, List<Adt2>))), R<(u64, (Adt1, List<Adt2>))>>(Ok(v2))
                } else {
                    let v3 = m308 - 1;
                    Ok::<(u64, (u64, (Adt1, List<Adt2>))), R<(u64, (Adt1, List<Adt2>))>>((v3, {
                        let c309: Fn7 = v0.clone();
                        c309.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m310 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f33(v0: u32, v1: u64, v2: u64, v3: Adt1, v4: u64, v5: Bytes, v6: (u64, (Adt1, List<Adt2>))) -> R<(u64, (Adt1, List<Adt2>))> {
    let v7: u64 = {
        let (h313, _) = v6.clone();
        h313
    };
    let v8: Adt3 = f37(f34(v3, v0, v5, v4, v1, v2, v7, {
        let (h314, _) = {
            let (_, h315) = v6.clone();
            h315
        };
        h314
    }, 64u64, f36(v7))?)?;
    Ok(({
        let a316 = v7;
        let a317 = 64u64;
        nat_add(a316, a317)?
    }, (f39(match v8.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h318) => {
            h318
        }
    }), List::cons(Adt2::C0(v8), {
        let (_, h319) = {
            let (_, h320) = v6;
            h320
        };
        h319
    }))))
}

pub fn f34(_: Adt1, v1: u32, v2: Bytes, v3: u64, v4: u64, v5: u64, v6: u64, v7: Adt1, v8: u64, v9: u32) -> R<Adt7> {
    Ok(Adt7::C0(0u64, v5, 1u64, {
        let a321 = v6;
        let a322 = 64u64;
        let a323 = 0u64;
        nat_quot(a321, a322, a323)
    }, {
        let a326 = {
            let a324 = v5;
            let a325 = 1024u64;
            nat_mul(a324, a325)?
        };
        let a327 = v6;
        nat_add(a326, a327)?
    }, v7, f49(v2, v3, {
        let a328 = v4;
        let a329 = v6;
        nat_add(a328, a329)?
    })?, v5, f50(v8, 7u64), {
        let a330 = v1;
        let a331 = v9;
        fixed_u32::bit_or(a330, a331)
    }))
}

pub fn f35() -> u32 {
    2u32
}

pub fn f36(v0: u64) -> u32 {
    let a332 = v0;
    let a333 = 0u64;
    if nat_eq(a332, a333) {
        f51()
    } else {
        0u32
    }
}

pub fn f37(v0: Adt7) -> R<Adt3> {
    let v1: u32 = f52(match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, h334, _, _) => {
            h334
        }
    });
    let v2: u32 = f53(match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, h335, _, _) => {
            h335
        }
    });
    Ok(Adt3::C0(match v0.clone() {
        Adt7::C0(h336, _, _, _, _, _, _, _, _, _) => {
            h336
        }
    }, match v0.clone() {
        Adt7::C0(_, h337, _, _, _, _, _, _, _, _) => {
            h337
        }
    }, match v0.clone() {
        Adt7::C0(_, _, h338, _, _, _, _, _, _, _) => {
            h338
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, h339, _, _, _, _, _, _) => {
            h339
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, h340, _, _, _, _, _) => {
            h340
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, h341, _, _, _, _) => {
            h341
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, h342, _, _, _) => {
            h342
        }
    }, v1, v2, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, _, h343, _) => {
            h343
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, _, _, h344) => {
            h344
        }
    }, f54(match v0.clone() {
        Adt7::C0(_, _, _, _, _, h345, _, _, _, _) => {
            h345
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, h346, _, _, _) => {
            h346
        }
    }, v1, v2, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, _, h347, _) => {
            h347
        }
    }, match v0 {
        Adt7::C0(_, _, _, _, _, _, _, _, _, h348) => {
            h348
        }
    })?))
}

pub fn f38(v0: Adt5, v1: Adt5, v2: Adt1, v3: u32) -> R<Adt7> {
    Ok(Adt7::C0(1u64, match v0.clone() {
        Adt5::C0(_, h349, _) => {
            h349
        }
    }, {
        let a352 = match v0.clone() {
            Adt5::C0(_, _, h350) => {
                h350
            }
        };
        let a353 = match v1.clone() {
            Adt5::C0(_, _, h351) => {
                h351
            }
        };
        nat_add(a352, a353)?
    }, 0u64, 0u64, v2, Adt4::C0({
        let m356 = match v0.clone() {
            Adt5::C0(h355, _, _) => {
                h355
            }
        };
        match m356 {
            Adt1::C0(h354, _, _, _, _, _, _, _) => {
                h354
            }
        }
    }, {
        let m359 = match v0.clone() {
            Adt5::C0(h358, _, _) => {
                h358
            }
        };
        match m359 {
            Adt1::C0(_, h357, _, _, _, _, _, _) => {
                h357
            }
        }
    }, {
        let m362 = match v0.clone() {
            Adt5::C0(h361, _, _) => {
                h361
            }
        };
        match m362 {
            Adt1::C0(_, _, h360, _, _, _, _, _) => {
                h360
            }
        }
    }, {
        let m365 = match v0.clone() {
            Adt5::C0(h364, _, _) => {
                h364
            }
        };
        match m365 {
            Adt1::C0(_, _, _, h363, _, _, _, _) => {
                h363
            }
        }
    }, {
        let m368 = match v0.clone() {
            Adt5::C0(h367, _, _) => {
                h367
            }
        };
        match m368 {
            Adt1::C0(_, _, _, _, h366, _, _, _) => {
                h366
            }
        }
    }, {
        let m371 = match v0.clone() {
            Adt5::C0(h370, _, _) => {
                h370
            }
        };
        match m371 {
            Adt1::C0(_, _, _, _, _, h369, _, _) => {
                h369
            }
        }
    }, {
        let m374 = match v0.clone() {
            Adt5::C0(h373, _, _) => {
                h373
            }
        };
        match m374 {
            Adt1::C0(_, _, _, _, _, _, h372, _) => {
                h372
            }
        }
    }, {
        let m377 = match v0 {
            Adt5::C0(h376, _, _) => {
                h376
            }
        };
        match m377 {
            Adt1::C0(_, _, _, _, _, _, _, h375) => {
                h375
            }
        }
    }, {
        let m380 = match v1.clone() {
            Adt5::C0(h379, _, _) => {
                h379
            }
        };
        match m380 {
            Adt1::C0(h378, _, _, _, _, _, _, _) => {
                h378
            }
        }
    }, {
        let m383 = match v1.clone() {
            Adt5::C0(h382, _, _) => {
                h382
            }
        };
        match m383 {
            Adt1::C0(_, h381, _, _, _, _, _, _) => {
                h381
            }
        }
    }, {
        let m386 = match v1.clone() {
            Adt5::C0(h385, _, _) => {
                h385
            }
        };
        match m386 {
            Adt1::C0(_, _, h384, _, _, _, _, _) => {
                h384
            }
        }
    }, {
        let m389 = match v1.clone() {
            Adt5::C0(h388, _, _) => {
                h388
            }
        };
        match m389 {
            Adt1::C0(_, _, _, h387, _, _, _, _) => {
                h387
            }
        }
    }, {
        let m392 = match v1.clone() {
            Adt5::C0(h391, _, _) => {
                h391
            }
        };
        match m392 {
            Adt1::C0(_, _, _, _, h390, _, _, _) => {
                h390
            }
        }
    }, {
        let m395 = match v1.clone() {
            Adt5::C0(h394, _, _) => {
                h394
            }
        };
        match m395 {
            Adt1::C0(_, _, _, _, _, h393, _, _) => {
                h393
            }
        }
    }, {
        let m398 = match v1.clone() {
            Adt5::C0(h397, _, _) => {
                h397
            }
        };
        match m398 {
            Adt1::C0(_, _, _, _, _, _, h396, _) => {
                h396
            }
        }
    }, {
        let m401 = match v1 {
            Adt5::C0(h400, _, _) => {
                h400
            }
        };
        match m401 {
            Adt1::C0(_, _, _, _, _, _, _, h399) => {
                h399
            }
        }
    }), 0u64, 64u32, {
        let a402 = v3;
        let a403 = f55();
        fixed_u32::bit_or(a402, a403)
    }))
}

pub fn f39(v0: Adt4) -> Adt1 {
    Adt1::C0(match v0.clone() {
        Adt4::C0(h404, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h404
        }
    }, match v0.clone() {
        Adt4::C0(_, h405, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h405
        }
    }, match v0.clone() {
        Adt4::C0(_, _, h406, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h406
        }
    }, match v0.clone() {
        Adt4::C0(_, _, _, h407, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h407
        }
    }, match v0.clone() {
        Adt4::C0(_, _, _, _, h408, _, _, _, _, _, _, _, _, _, _, _) => {
            h408
        }
    }, match v0.clone() {
        Adt4::C0(_, _, _, _, _, h409, _, _, _, _, _, _, _, _, _, _) => {
            h409
        }
    }, match v0.clone() {
        Adt4::C0(_, _, _, _, _, _, h410, _, _, _, _, _, _, _, _, _) => {
            h410
        }
    }, match v0 {
        Adt4::C0(_, _, _, _, _, _, _, h411, _, _, _, _, _, _, _, _) => {
            h411
        }
    })
}

pub fn f40(v0: Adt7, v1: u64) -> Adt7 {
    Adt7::C0(2u64, match v0.clone() {
        Adt7::C0(_, h412, _, _, _, _, _, _, _, _) => {
            h412
        }
    }, match v0.clone() {
        Adt7::C0(_, _, h413, _, _, _, _, _, _, _) => {
            h413
        }
    }, v1, match v0.clone() {
        Adt7::C0(_, _, _, _, h414, _, _, _, _, _) => {
            h414
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, h415, _, _, _, _) => {
            h415
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, h416, _, _, _) => {
            h416
        }
    }, v1, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, _, h417, _) => {
            h417
        }
    }, {
        let a419 = match v0 {
            Adt7::C0(_, _, _, _, _, _, _, _, _, h418) => {
                h418
            }
        };
        let a420 = f56();
        fixed_u32::bit_or(a419, a420)
    })
}

pub fn f41() -> u32 {
    16u32
}

pub fn f42() -> u32 {
    64u32
}

pub fn f43() -> u32 {
    32u32
}

pub fn f44(v0: Fn8, p424: List<Adt2>, p425: List<Adt2>) -> List<Adt2> {
    let m423 = (p424, p425);
    {
        let mut m423 = m423;
        loop {
            let step = {
                let (v1, v2) = m423;
                let m421 = v2;
                match m421.uncons() {
                    None => {
                        Err::<(List<Adt2>, List<Adt2>), List<Adt2>>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(List<Adt2>, List<Adt2>), List<Adt2>>(({
                            let c422: Fn8 = v0.clone();
                            c422.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m423 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f45(v0: List<Adt2>, v1: Adt2) -> List<Adt2> {
    List::cons(v1, v0)
}

pub fn f46(v0: Adt1, v1: u32, v2: Adt8) -> R<(List<Adt5>, List<Adt2>)> {
    let v3: Adt8 = {
        let (h426, _) = f57(Fn9::F58(v1, v0), 64u64, v2)?;
        h426
    };
    let v4: List<Adt5> = List::cons(match v3.clone() {
        Adt8::C0(h427, _, _, _) => {
            h427
        }
    }, match v3.clone() {
        Adt8::C0(_, _, h428, _) => {
            h428
        }
    });
    Ok((v4.clone(), List::cons(Adt2::C1(match v3.clone() {
        Adt8::C0(h429, _, _, _) => {
            h429
        }
    }, {
        let a430 = v4;
        length_list(a430)
    }), match v3 {
        Adt8::C0(_, _, _, h431) => {
            h431
        }
    })))
}

pub fn f47(v0: u32) -> Bytes {
    let a442 = f59({
        let a432 = v0;
        let a433 = 255u32;
        fixed_u32::bit_and(a432, a433)
    });
    let a443 = {
        let a440 = f59({
            let a434 = f27(v0, 8u32);
            let a435 = 255u32;
            fixed_u32::bit_and(a434, a435)
        });
        let a441 = {
            let a438 = f59({
                let a436 = f27(v0, 16u32);
                let a437 = 255u32;
                fixed_u32::bit_and(a436, a437)
            });
            let a439 = f59(f27(v0, 24u32));
            append_bytes(a438, a439)
        };
        append_bytes(a440, a441)
    };
    append_bytes(a442, a443)
}

pub fn f48(v0: List<Bytes>) -> List<Bytes> {
    let v1: (Option<Bytes>, List<Bytes>) = f60(Fn10::F61, (None::<Bytes>, List::<Bytes>::nil()), v0);
    let v3: List<Bytes> = {
        let m445 = {
            let (h444, _) = v1;
            h444
        };
        match m445 {
            None => {
                let (_, h447) = v1;
                h447
            }
            Some(v2) => {
                List::cons(v2, {
                    let (_, h446) = v1;
                    h446
                })
            }
        }
    };
    f14(Fn2::F62, List::<Bytes>::nil(), v3)
}

pub fn f49(v0: Bytes, v1: u64, v2: u64) -> R<Adt4> {
    Ok(Adt4::C0(f63(v0.clone(), v1, {
        let a448 = v2;
        let a449 = 0u64;
        nat_add(a448, a449)?
    })?, f63(v0.clone(), v1, {
        let a450 = v2;
        let a451 = 4u64;
        nat_add(a450, a451)?
    })?, f63(v0.clone(), v1, {
        let a452 = v2;
        let a453 = 8u64;
        nat_add(a452, a453)?
    })?, f63(v0.clone(), v1, {
        let a454 = v2;
        let a455 = 12u64;
        nat_add(a454, a455)?
    })?, f63(v0.clone(), v1, {
        let a456 = v2;
        let a457 = 16u64;
        nat_add(a456, a457)?
    })?, f63(v0.clone(), v1, {
        let a458 = v2;
        let a459 = 20u64;
        nat_add(a458, a459)?
    })?, f63(v0.clone(), v1, {
        let a460 = v2;
        let a461 = 24u64;
        nat_add(a460, a461)?
    })?, f63(v0.clone(), v1, {
        let a462 = v2;
        let a463 = 28u64;
        nat_add(a462, a463)?
    })?, f63(v0.clone(), v1, {
        let a464 = v2;
        let a465 = 32u64;
        nat_add(a464, a465)?
    })?, f63(v0.clone(), v1, {
        let a466 = v2;
        let a467 = 36u64;
        nat_add(a466, a467)?
    })?, f63(v0.clone(), v1, {
        let a468 = v2;
        let a469 = 40u64;
        nat_add(a468, a469)?
    })?, f63(v0.clone(), v1, {
        let a470 = v2;
        let a471 = 44u64;
        nat_add(a470, a471)?
    })?, f63(v0.clone(), v1, {
        let a472 = v2;
        let a473 = 48u64;
        nat_add(a472, a473)?
    })?, f63(v0.clone(), v1, {
        let a474 = v2;
        let a475 = 52u64;
        nat_add(a474, a475)?
    })?, f63(v0.clone(), v1, {
        let a476 = v2;
        let a477 = 56u64;
        nat_add(a476, a477)?
    })?, f63(v0, v1, {
        let a478 = v2;
        let a479 = 60u64;
        nat_add(a478, a479)?
    })?))
}

pub fn f50(v0: u64, v1: u64) -> u32 {
    let (_, h480) = {
        let (_, h481) = f64(Fn11::F65, v1, (v0, (1u32, 0u32)));
        h481
    };
    h480
}

pub fn f51() -> u32 {
    1u32
}

pub fn f52(v0: u64) -> u32 {
    f50({
        let a482 = v0;
        let a483 = 4294967296u64;
        let a484 = 0u64;
        nat_rem(a482, a483, a484)
    }, 32u64)
}

pub fn f53(v0: u64) -> u32 {
    let a485 = v0;
    let a486 = 4294967296u64;
    if nat_lt(a485, a486) {
        0u32
    } else {
        f50({
            let a487 = v0;
            let a488 = 4294967296u64;
            let a489 = 0u64;
            nat_quot(a487, a488, a489)
        }, 32u64)
    }
}

pub fn f54(v0: Adt1, v1: Adt4, v2: u32, v3: u32, v4: u32, v5: u32) -> R<Adt4> {
    Ok(f66(f67(v0, v1, v2, v3, v4, v5)?))
}

pub fn f55() -> u32 {
    4u32
}

pub fn f56() -> u32 {
    8u32
}

pub fn f57(v0: Fn9, p494: u64, p495: Adt8) -> R<(Adt8, bool)> {
    let m493 = (p494, p495);
    {
        let mut m493 = m493;
        loop {
            let step = {
                let (v1, v2) = m493;
                let m490 = v1;
                if m490 == 0 {
                    Err::<(u64, Adt8), R<(Adt8, bool)>>(Ok((v2, false)))
                } else {
                    let v3 = m490 - 1;
                    let m492 = {
                        let c491: Fn9 = v0.clone();
                        c491.apply(v2.clone())?
                    };
                    match m492 {
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
                Ok(next) => { m493 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f58(v0: u32, v1: Adt1, v2: Adt8) -> R<Option<Adt8>> {
    f68(v1, v0, v2)
}

pub fn f59(v0: u32) -> Bytes {
    let a498 = {
        let a496 = v0;
        let a497 = 128u32;
        fixed_u32::bit_and(a496, a497)
    };
    let a499 = 0u32;
    if equal(a498, a499) {
        let a502 = {
            let a500 = v0;
            let a501 = 64u32;
            fixed_u32::bit_and(a500, a501)
        };
        let a503 = 0u32;
        if equal(a502, a503) {
            let a506 = {
                let a504 = v0;
                let a505 = 32u32;
                fixed_u32::bit_and(a504, a505)
            };
            let a507 = 0u32;
            if equal(a506, a507) {
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
                                    Bytes::lit(&[0x00])
                                } else {
                                    Bytes::lit(&[0x01])
                                }
                            } else {
                                let a530 = {
                                    let a528 = v0;
                                    let a529 = 1u32;
                                    fixed_u32::bit_and(a528, a529)
                                };
                                let a531 = 0u32;
                                if equal(a530, a531) {
                                    Bytes::lit(&[0x02])
                                } else {
                                    Bytes::lit(&[0x03])
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
                                    Bytes::lit(&[0x04])
                                } else {
                                    Bytes::lit(&[0x05])
                                }
                            } else {
                                let a542 = {
                                    let a540 = v0;
                                    let a541 = 1u32;
                                    fixed_u32::bit_and(a540, a541)
                                };
                                let a543 = 0u32;
                                if equal(a542, a543) {
                                    Bytes::lit(&[0x06])
                                } else {
                                    Bytes::lit(&[0x07])
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
                                    Bytes::lit(&[0x08])
                                } else {
                                    Bytes::lit(&[0x09])
                                }
                            } else {
                                let a558 = {
                                    let a556 = v0;
                                    let a557 = 1u32;
                                    fixed_u32::bit_and(a556, a557)
                                };
                                let a559 = 0u32;
                                if equal(a558, a559) {
                                    Bytes::lit(&[0x0a])
                                } else {
                                    Bytes::lit(&[0x0b])
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
                                    Bytes::lit(&[0x0c])
                                } else {
                                    Bytes::lit(&[0x0d])
                                }
                            } else {
                                let a570 = {
                                    let a568 = v0;
                                    let a569 = 1u32;
                                    fixed_u32::bit_and(a568, a569)
                                };
                                let a571 = 0u32;
                                if equal(a570, a571) {
                                    Bytes::lit(&[0x0e])
                                } else {
                                    Bytes::lit(&[0x0f])
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
                                    Bytes::lit(&[0x10])
                                } else {
                                    Bytes::lit(&[0x11])
                                }
                            } else {
                                let a590 = {
                                    let a588 = v0;
                                    let a589 = 1u32;
                                    fixed_u32::bit_and(a588, a589)
                                };
                                let a591 = 0u32;
                                if equal(a590, a591) {
                                    Bytes::lit(&[0x12])
                                } else {
                                    Bytes::lit(&[0x13])
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
                                    Bytes::lit(&[0x14])
                                } else {
                                    Bytes::lit(&[0x15])
                                }
                            } else {
                                let a602 = {
                                    let a600 = v0;
                                    let a601 = 1u32;
                                    fixed_u32::bit_and(a600, a601)
                                };
                                let a603 = 0u32;
                                if equal(a602, a603) {
                                    Bytes::lit(&[0x16])
                                } else {
                                    Bytes::lit(&[0x17])
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
                                    Bytes::lit(&[0x18])
                                } else {
                                    Bytes::lit(&[0x19])
                                }
                            } else {
                                let a618 = {
                                    let a616 = v0;
                                    let a617 = 1u32;
                                    fixed_u32::bit_and(a616, a617)
                                };
                                let a619 = 0u32;
                                if equal(a618, a619) {
                                    Bytes::lit(&[0x1a])
                                } else {
                                    Bytes::lit(&[0x1b])
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
                                    Bytes::lit(&[0x1c])
                                } else {
                                    Bytes::lit(&[0x1d])
                                }
                            } else {
                                let a630 = {
                                    let a628 = v0;
                                    let a629 = 1u32;
                                    fixed_u32::bit_and(a628, a629)
                                };
                                let a631 = 0u32;
                                if equal(a630, a631) {
                                    Bytes::lit(&[0x1e])
                                } else {
                                    Bytes::lit(&[0x1f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a634 = {
                    let a632 = v0;
                    let a633 = 16u32;
                    fixed_u32::bit_and(a632, a633)
                };
                let a635 = 0u32;
                if equal(a634, a635) {
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
                                    Bytes::lit(&[0x20])
                                } else {
                                    Bytes::lit(&[0x21])
                                }
                            } else {
                                let a654 = {
                                    let a652 = v0;
                                    let a653 = 1u32;
                                    fixed_u32::bit_and(a652, a653)
                                };
                                let a655 = 0u32;
                                if equal(a654, a655) {
                                    Bytes::lit(&[0x22])
                                } else {
                                    Bytes::lit(&[0x23])
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
                                    Bytes::lit(&[0x24])
                                } else {
                                    Bytes::lit(&[0x25])
                                }
                            } else {
                                let a666 = {
                                    let a664 = v0;
                                    let a665 = 1u32;
                                    fixed_u32::bit_and(a664, a665)
                                };
                                let a667 = 0u32;
                                if equal(a666, a667) {
                                    Bytes::lit(&[0x26])
                                } else {
                                    Bytes::lit(&[0x27])
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
                                    Bytes::lit(&[0x28])
                                } else {
                                    Bytes::lit(&[0x29])
                                }
                            } else {
                                let a682 = {
                                    let a680 = v0;
                                    let a681 = 1u32;
                                    fixed_u32::bit_and(a680, a681)
                                };
                                let a683 = 0u32;
                                if equal(a682, a683) {
                                    Bytes::lit(&[0x2a])
                                } else {
                                    Bytes::lit(&[0x2b])
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
                                    Bytes::lit(&[0x2c])
                                } else {
                                    Bytes::lit(&[0x2d])
                                }
                            } else {
                                let a694 = {
                                    let a692 = v0;
                                    let a693 = 1u32;
                                    fixed_u32::bit_and(a692, a693)
                                };
                                let a695 = 0u32;
                                if equal(a694, a695) {
                                    Bytes::lit(&[0x2e])
                                } else {
                                    Bytes::lit(&[0x2f])
                                }
                            }
                        }
                    }
                } else {
                    let a698 = {
                        let a696 = v0;
                        let a697 = 8u32;
                        fixed_u32::bit_and(a696, a697)
                    };
                    let a699 = 0u32;
                    if equal(a698, a699) {
                        let a702 = {
                            let a700 = v0;
                            let a701 = 4u32;
                            fixed_u32::bit_and(a700, a701)
                        };
                        let a703 = 0u32;
                        if equal(a702, a703) {
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
                                    Bytes::lit(&[0x30])
                                } else {
                                    Bytes::lit(&[0x31])
                                }
                            } else {
                                let a714 = {
                                    let a712 = v0;
                                    let a713 = 1u32;
                                    fixed_u32::bit_and(a712, a713)
                                };
                                let a715 = 0u32;
                                if equal(a714, a715) {
                                    Bytes::lit(&[0x32])
                                } else {
                                    Bytes::lit(&[0x33])
                                }
                            }
                        } else {
                            let a718 = {
                                let a716 = v0;
                                let a717 = 2u32;
                                fixed_u32::bit_and(a716, a717)
                            };
                            let a719 = 0u32;
                            if equal(a718, a719) {
                                let a722 = {
                                    let a720 = v0;
                                    let a721 = 1u32;
                                    fixed_u32::bit_and(a720, a721)
                                };
                                let a723 = 0u32;
                                if equal(a722, a723) {
                                    Bytes::lit(&[0x34])
                                } else {
                                    Bytes::lit(&[0x35])
                                }
                            } else {
                                let a726 = {
                                    let a724 = v0;
                                    let a725 = 1u32;
                                    fixed_u32::bit_and(a724, a725)
                                };
                                let a727 = 0u32;
                                if equal(a726, a727) {
                                    Bytes::lit(&[0x36])
                                } else {
                                    Bytes::lit(&[0x37])
                                }
                            }
                        }
                    } else {
                        let a730 = {
                            let a728 = v0;
                            let a729 = 4u32;
                            fixed_u32::bit_and(a728, a729)
                        };
                        let a731 = 0u32;
                        if equal(a730, a731) {
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
                                    Bytes::lit(&[0x38])
                                } else {
                                    Bytes::lit(&[0x39])
                                }
                            } else {
                                let a742 = {
                                    let a740 = v0;
                                    let a741 = 1u32;
                                    fixed_u32::bit_and(a740, a741)
                                };
                                let a743 = 0u32;
                                if equal(a742, a743) {
                                    Bytes::lit(&[0x3a])
                                } else {
                                    Bytes::lit(&[0x3b])
                                }
                            }
                        } else {
                            let a746 = {
                                let a744 = v0;
                                let a745 = 2u32;
                                fixed_u32::bit_and(a744, a745)
                            };
                            let a747 = 0u32;
                            if equal(a746, a747) {
                                let a750 = {
                                    let a748 = v0;
                                    let a749 = 1u32;
                                    fixed_u32::bit_and(a748, a749)
                                };
                                let a751 = 0u32;
                                if equal(a750, a751) {
                                    Bytes::lit(&[0x3c])
                                } else {
                                    Bytes::lit(&[0x3d])
                                }
                            } else {
                                let a754 = {
                                    let a752 = v0;
                                    let a753 = 1u32;
                                    fixed_u32::bit_and(a752, a753)
                                };
                                let a755 = 0u32;
                                if equal(a754, a755) {
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
            let a758 = {
                let a756 = v0;
                let a757 = 32u32;
                fixed_u32::bit_and(a756, a757)
            };
            let a759 = 0u32;
            if equal(a758, a759) {
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
                                    Bytes::lit(&[0x40])
                                } else {
                                    Bytes::lit(&[0x41])
                                }
                            } else {
                                let a782 = {
                                    let a780 = v0;
                                    let a781 = 1u32;
                                    fixed_u32::bit_and(a780, a781)
                                };
                                let a783 = 0u32;
                                if equal(a782, a783) {
                                    Bytes::lit(&[0x42])
                                } else {
                                    Bytes::lit(&[0x43])
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
                                    Bytes::lit(&[0x44])
                                } else {
                                    Bytes::lit(&[0x45])
                                }
                            } else {
                                let a794 = {
                                    let a792 = v0;
                                    let a793 = 1u32;
                                    fixed_u32::bit_and(a792, a793)
                                };
                                let a795 = 0u32;
                                if equal(a794, a795) {
                                    Bytes::lit(&[0x46])
                                } else {
                                    Bytes::lit(&[0x47])
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
                                    Bytes::lit(&[0x48])
                                } else {
                                    Bytes::lit(&[0x49])
                                }
                            } else {
                                let a810 = {
                                    let a808 = v0;
                                    let a809 = 1u32;
                                    fixed_u32::bit_and(a808, a809)
                                };
                                let a811 = 0u32;
                                if equal(a810, a811) {
                                    Bytes::lit(&[0x4a])
                                } else {
                                    Bytes::lit(&[0x4b])
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
                                    Bytes::lit(&[0x4c])
                                } else {
                                    Bytes::lit(&[0x4d])
                                }
                            } else {
                                let a822 = {
                                    let a820 = v0;
                                    let a821 = 1u32;
                                    fixed_u32::bit_and(a820, a821)
                                };
                                let a823 = 0u32;
                                if equal(a822, a823) {
                                    Bytes::lit(&[0x4e])
                                } else {
                                    Bytes::lit(&[0x4f])
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
                                    Bytes::lit(&[0x50])
                                } else {
                                    Bytes::lit(&[0x51])
                                }
                            } else {
                                let a842 = {
                                    let a840 = v0;
                                    let a841 = 1u32;
                                    fixed_u32::bit_and(a840, a841)
                                };
                                let a843 = 0u32;
                                if equal(a842, a843) {
                                    Bytes::lit(&[0x52])
                                } else {
                                    Bytes::lit(&[0x53])
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
                                    Bytes::lit(&[0x54])
                                } else {
                                    Bytes::lit(&[0x55])
                                }
                            } else {
                                let a854 = {
                                    let a852 = v0;
                                    let a853 = 1u32;
                                    fixed_u32::bit_and(a852, a853)
                                };
                                let a855 = 0u32;
                                if equal(a854, a855) {
                                    Bytes::lit(&[0x56])
                                } else {
                                    Bytes::lit(&[0x57])
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
                                    Bytes::lit(&[0x58])
                                } else {
                                    Bytes::lit(&[0x59])
                                }
                            } else {
                                let a870 = {
                                    let a868 = v0;
                                    let a869 = 1u32;
                                    fixed_u32::bit_and(a868, a869)
                                };
                                let a871 = 0u32;
                                if equal(a870, a871) {
                                    Bytes::lit(&[0x5a])
                                } else {
                                    Bytes::lit(&[0x5b])
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
                                    Bytes::lit(&[0x5c])
                                } else {
                                    Bytes::lit(&[0x5d])
                                }
                            } else {
                                let a882 = {
                                    let a880 = v0;
                                    let a881 = 1u32;
                                    fixed_u32::bit_and(a880, a881)
                                };
                                let a883 = 0u32;
                                if equal(a882, a883) {
                                    Bytes::lit(&[0x5e])
                                } else {
                                    Bytes::lit(&[0x5f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a886 = {
                    let a884 = v0;
                    let a885 = 16u32;
                    fixed_u32::bit_and(a884, a885)
                };
                let a887 = 0u32;
                if equal(a886, a887) {
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
                                    Bytes::lit(&[0x60])
                                } else {
                                    Bytes::lit(&[0x61])
                                }
                            } else {
                                let a906 = {
                                    let a904 = v0;
                                    let a905 = 1u32;
                                    fixed_u32::bit_and(a904, a905)
                                };
                                let a907 = 0u32;
                                if equal(a906, a907) {
                                    Bytes::lit(&[0x62])
                                } else {
                                    Bytes::lit(&[0x63])
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
                                    Bytes::lit(&[0x64])
                                } else {
                                    Bytes::lit(&[0x65])
                                }
                            } else {
                                let a918 = {
                                    let a916 = v0;
                                    let a917 = 1u32;
                                    fixed_u32::bit_and(a916, a917)
                                };
                                let a919 = 0u32;
                                if equal(a918, a919) {
                                    Bytes::lit(&[0x66])
                                } else {
                                    Bytes::lit(&[0x67])
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
                                    Bytes::lit(&[0x68])
                                } else {
                                    Bytes::lit(&[0x69])
                                }
                            } else {
                                let a934 = {
                                    let a932 = v0;
                                    let a933 = 1u32;
                                    fixed_u32::bit_and(a932, a933)
                                };
                                let a935 = 0u32;
                                if equal(a934, a935) {
                                    Bytes::lit(&[0x6a])
                                } else {
                                    Bytes::lit(&[0x6b])
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
                                    Bytes::lit(&[0x6c])
                                } else {
                                    Bytes::lit(&[0x6d])
                                }
                            } else {
                                let a946 = {
                                    let a944 = v0;
                                    let a945 = 1u32;
                                    fixed_u32::bit_and(a944, a945)
                                };
                                let a947 = 0u32;
                                if equal(a946, a947) {
                                    Bytes::lit(&[0x6e])
                                } else {
                                    Bytes::lit(&[0x6f])
                                }
                            }
                        }
                    }
                } else {
                    let a950 = {
                        let a948 = v0;
                        let a949 = 8u32;
                        fixed_u32::bit_and(a948, a949)
                    };
                    let a951 = 0u32;
                    if equal(a950, a951) {
                        let a954 = {
                            let a952 = v0;
                            let a953 = 4u32;
                            fixed_u32::bit_and(a952, a953)
                        };
                        let a955 = 0u32;
                        if equal(a954, a955) {
                            let a958 = {
                                let a956 = v0;
                                let a957 = 2u32;
                                fixed_u32::bit_and(a956, a957)
                            };
                            let a959 = 0u32;
                            if equal(a958, a959) {
                                let a962 = {
                                    let a960 = v0;
                                    let a961 = 1u32;
                                    fixed_u32::bit_and(a960, a961)
                                };
                                let a963 = 0u32;
                                if equal(a962, a963) {
                                    Bytes::lit(&[0x70])
                                } else {
                                    Bytes::lit(&[0x71])
                                }
                            } else {
                                let a966 = {
                                    let a964 = v0;
                                    let a965 = 1u32;
                                    fixed_u32::bit_and(a964, a965)
                                };
                                let a967 = 0u32;
                                if equal(a966, a967) {
                                    Bytes::lit(&[0x72])
                                } else {
                                    Bytes::lit(&[0x73])
                                }
                            }
                        } else {
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
                                    Bytes::lit(&[0x74])
                                } else {
                                    Bytes::lit(&[0x75])
                                }
                            } else {
                                let a978 = {
                                    let a976 = v0;
                                    let a977 = 1u32;
                                    fixed_u32::bit_and(a976, a977)
                                };
                                let a979 = 0u32;
                                if equal(a978, a979) {
                                    Bytes::lit(&[0x76])
                                } else {
                                    Bytes::lit(&[0x77])
                                }
                            }
                        }
                    } else {
                        let a982 = {
                            let a980 = v0;
                            let a981 = 4u32;
                            fixed_u32::bit_and(a980, a981)
                        };
                        let a983 = 0u32;
                        if equal(a982, a983) {
                            let a986 = {
                                let a984 = v0;
                                let a985 = 2u32;
                                fixed_u32::bit_and(a984, a985)
                            };
                            let a987 = 0u32;
                            if equal(a986, a987) {
                                let a990 = {
                                    let a988 = v0;
                                    let a989 = 1u32;
                                    fixed_u32::bit_and(a988, a989)
                                };
                                let a991 = 0u32;
                                if equal(a990, a991) {
                                    Bytes::lit(&[0x78])
                                } else {
                                    Bytes::lit(&[0x79])
                                }
                            } else {
                                let a994 = {
                                    let a992 = v0;
                                    let a993 = 1u32;
                                    fixed_u32::bit_and(a992, a993)
                                };
                                let a995 = 0u32;
                                if equal(a994, a995) {
                                    Bytes::lit(&[0x7a])
                                } else {
                                    Bytes::lit(&[0x7b])
                                }
                            }
                        } else {
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
                                    Bytes::lit(&[0x7c])
                                } else {
                                    Bytes::lit(&[0x7d])
                                }
                            } else {
                                let a1006 = {
                                    let a1004 = v0;
                                    let a1005 = 1u32;
                                    fixed_u32::bit_and(a1004, a1005)
                                };
                                let a1007 = 0u32;
                                if equal(a1006, a1007) {
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
        let a1010 = {
            let a1008 = v0;
            let a1009 = 64u32;
            fixed_u32::bit_and(a1008, a1009)
        };
        let a1011 = 0u32;
        if equal(a1010, a1011) {
            let a1014 = {
                let a1012 = v0;
                let a1013 = 32u32;
                fixed_u32::bit_and(a1012, a1013)
            };
            let a1015 = 0u32;
            if equal(a1014, a1015) {
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
                                    Bytes::lit(&[0x80])
                                } else {
                                    Bytes::lit(&[0x81])
                                }
                            } else {
                                let a1038 = {
                                    let a1036 = v0;
                                    let a1037 = 1u32;
                                    fixed_u32::bit_and(a1036, a1037)
                                };
                                let a1039 = 0u32;
                                if equal(a1038, a1039) {
                                    Bytes::lit(&[0x82])
                                } else {
                                    Bytes::lit(&[0x83])
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
                                    Bytes::lit(&[0x84])
                                } else {
                                    Bytes::lit(&[0x85])
                                }
                            } else {
                                let a1050 = {
                                    let a1048 = v0;
                                    let a1049 = 1u32;
                                    fixed_u32::bit_and(a1048, a1049)
                                };
                                let a1051 = 0u32;
                                if equal(a1050, a1051) {
                                    Bytes::lit(&[0x86])
                                } else {
                                    Bytes::lit(&[0x87])
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
                                    Bytes::lit(&[0x88])
                                } else {
                                    Bytes::lit(&[0x89])
                                }
                            } else {
                                let a1066 = {
                                    let a1064 = v0;
                                    let a1065 = 1u32;
                                    fixed_u32::bit_and(a1064, a1065)
                                };
                                let a1067 = 0u32;
                                if equal(a1066, a1067) {
                                    Bytes::lit(&[0x8a])
                                } else {
                                    Bytes::lit(&[0x8b])
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
                                    Bytes::lit(&[0x8c])
                                } else {
                                    Bytes::lit(&[0x8d])
                                }
                            } else {
                                let a1078 = {
                                    let a1076 = v0;
                                    let a1077 = 1u32;
                                    fixed_u32::bit_and(a1076, a1077)
                                };
                                let a1079 = 0u32;
                                if equal(a1078, a1079) {
                                    Bytes::lit(&[0x8e])
                                } else {
                                    Bytes::lit(&[0x8f])
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
                                    Bytes::lit(&[0x90])
                                } else {
                                    Bytes::lit(&[0x91])
                                }
                            } else {
                                let a1098 = {
                                    let a1096 = v0;
                                    let a1097 = 1u32;
                                    fixed_u32::bit_and(a1096, a1097)
                                };
                                let a1099 = 0u32;
                                if equal(a1098, a1099) {
                                    Bytes::lit(&[0x92])
                                } else {
                                    Bytes::lit(&[0x93])
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
                                    Bytes::lit(&[0x94])
                                } else {
                                    Bytes::lit(&[0x95])
                                }
                            } else {
                                let a1110 = {
                                    let a1108 = v0;
                                    let a1109 = 1u32;
                                    fixed_u32::bit_and(a1108, a1109)
                                };
                                let a1111 = 0u32;
                                if equal(a1110, a1111) {
                                    Bytes::lit(&[0x96])
                                } else {
                                    Bytes::lit(&[0x97])
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
                                    Bytes::lit(&[0x98])
                                } else {
                                    Bytes::lit(&[0x99])
                                }
                            } else {
                                let a1126 = {
                                    let a1124 = v0;
                                    let a1125 = 1u32;
                                    fixed_u32::bit_and(a1124, a1125)
                                };
                                let a1127 = 0u32;
                                if equal(a1126, a1127) {
                                    Bytes::lit(&[0x9a])
                                } else {
                                    Bytes::lit(&[0x9b])
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
                                    Bytes::lit(&[0x9c])
                                } else {
                                    Bytes::lit(&[0x9d])
                                }
                            } else {
                                let a1138 = {
                                    let a1136 = v0;
                                    let a1137 = 1u32;
                                    fixed_u32::bit_and(a1136, a1137)
                                };
                                let a1139 = 0u32;
                                if equal(a1138, a1139) {
                                    Bytes::lit(&[0x9e])
                                } else {
                                    Bytes::lit(&[0x9f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a1142 = {
                    let a1140 = v0;
                    let a1141 = 16u32;
                    fixed_u32::bit_and(a1140, a1141)
                };
                let a1143 = 0u32;
                if equal(a1142, a1143) {
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
                                    Bytes::lit(&[0xa0])
                                } else {
                                    Bytes::lit(&[0xa1])
                                }
                            } else {
                                let a1162 = {
                                    let a1160 = v0;
                                    let a1161 = 1u32;
                                    fixed_u32::bit_and(a1160, a1161)
                                };
                                let a1163 = 0u32;
                                if equal(a1162, a1163) {
                                    Bytes::lit(&[0xa2])
                                } else {
                                    Bytes::lit(&[0xa3])
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
                                    Bytes::lit(&[0xa4])
                                } else {
                                    Bytes::lit(&[0xa5])
                                }
                            } else {
                                let a1174 = {
                                    let a1172 = v0;
                                    let a1173 = 1u32;
                                    fixed_u32::bit_and(a1172, a1173)
                                };
                                let a1175 = 0u32;
                                if equal(a1174, a1175) {
                                    Bytes::lit(&[0xa6])
                                } else {
                                    Bytes::lit(&[0xa7])
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
                                    Bytes::lit(&[0xa8])
                                } else {
                                    Bytes::lit(&[0xa9])
                                }
                            } else {
                                let a1190 = {
                                    let a1188 = v0;
                                    let a1189 = 1u32;
                                    fixed_u32::bit_and(a1188, a1189)
                                };
                                let a1191 = 0u32;
                                if equal(a1190, a1191) {
                                    Bytes::lit(&[0xaa])
                                } else {
                                    Bytes::lit(&[0xab])
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
                                    Bytes::lit(&[0xac])
                                } else {
                                    Bytes::lit(&[0xad])
                                }
                            } else {
                                let a1202 = {
                                    let a1200 = v0;
                                    let a1201 = 1u32;
                                    fixed_u32::bit_and(a1200, a1201)
                                };
                                let a1203 = 0u32;
                                if equal(a1202, a1203) {
                                    Bytes::lit(&[0xae])
                                } else {
                                    Bytes::lit(&[0xaf])
                                }
                            }
                        }
                    }
                } else {
                    let a1206 = {
                        let a1204 = v0;
                        let a1205 = 8u32;
                        fixed_u32::bit_and(a1204, a1205)
                    };
                    let a1207 = 0u32;
                    if equal(a1206, a1207) {
                        let a1210 = {
                            let a1208 = v0;
                            let a1209 = 4u32;
                            fixed_u32::bit_and(a1208, a1209)
                        };
                        let a1211 = 0u32;
                        if equal(a1210, a1211) {
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
                                    Bytes::lit(&[0xb0])
                                } else {
                                    Bytes::lit(&[0xb1])
                                }
                            } else {
                                let a1222 = {
                                    let a1220 = v0;
                                    let a1221 = 1u32;
                                    fixed_u32::bit_and(a1220, a1221)
                                };
                                let a1223 = 0u32;
                                if equal(a1222, a1223) {
                                    Bytes::lit(&[0xb2])
                                } else {
                                    Bytes::lit(&[0xb3])
                                }
                            }
                        } else {
                            let a1226 = {
                                let a1224 = v0;
                                let a1225 = 2u32;
                                fixed_u32::bit_and(a1224, a1225)
                            };
                            let a1227 = 0u32;
                            if equal(a1226, a1227) {
                                let a1230 = {
                                    let a1228 = v0;
                                    let a1229 = 1u32;
                                    fixed_u32::bit_and(a1228, a1229)
                                };
                                let a1231 = 0u32;
                                if equal(a1230, a1231) {
                                    Bytes::lit(&[0xb4])
                                } else {
                                    Bytes::lit(&[0xb5])
                                }
                            } else {
                                let a1234 = {
                                    let a1232 = v0;
                                    let a1233 = 1u32;
                                    fixed_u32::bit_and(a1232, a1233)
                                };
                                let a1235 = 0u32;
                                if equal(a1234, a1235) {
                                    Bytes::lit(&[0xb6])
                                } else {
                                    Bytes::lit(&[0xb7])
                                }
                            }
                        }
                    } else {
                        let a1238 = {
                            let a1236 = v0;
                            let a1237 = 4u32;
                            fixed_u32::bit_and(a1236, a1237)
                        };
                        let a1239 = 0u32;
                        if equal(a1238, a1239) {
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
                                    Bytes::lit(&[0xb8])
                                } else {
                                    Bytes::lit(&[0xb9])
                                }
                            } else {
                                let a1250 = {
                                    let a1248 = v0;
                                    let a1249 = 1u32;
                                    fixed_u32::bit_and(a1248, a1249)
                                };
                                let a1251 = 0u32;
                                if equal(a1250, a1251) {
                                    Bytes::lit(&[0xba])
                                } else {
                                    Bytes::lit(&[0xbb])
                                }
                            }
                        } else {
                            let a1254 = {
                                let a1252 = v0;
                                let a1253 = 2u32;
                                fixed_u32::bit_and(a1252, a1253)
                            };
                            let a1255 = 0u32;
                            if equal(a1254, a1255) {
                                let a1258 = {
                                    let a1256 = v0;
                                    let a1257 = 1u32;
                                    fixed_u32::bit_and(a1256, a1257)
                                };
                                let a1259 = 0u32;
                                if equal(a1258, a1259) {
                                    Bytes::lit(&[0xbc])
                                } else {
                                    Bytes::lit(&[0xbd])
                                }
                            } else {
                                let a1262 = {
                                    let a1260 = v0;
                                    let a1261 = 1u32;
                                    fixed_u32::bit_and(a1260, a1261)
                                };
                                let a1263 = 0u32;
                                if equal(a1262, a1263) {
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
            let a1266 = {
                let a1264 = v0;
                let a1265 = 32u32;
                fixed_u32::bit_and(a1264, a1265)
            };
            let a1267 = 0u32;
            if equal(a1266, a1267) {
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
                                    Bytes::lit(&[0xc0])
                                } else {
                                    Bytes::lit(&[0xc1])
                                }
                            } else {
                                let a1290 = {
                                    let a1288 = v0;
                                    let a1289 = 1u32;
                                    fixed_u32::bit_and(a1288, a1289)
                                };
                                let a1291 = 0u32;
                                if equal(a1290, a1291) {
                                    Bytes::lit(&[0xc2])
                                } else {
                                    Bytes::lit(&[0xc3])
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
                                    Bytes::lit(&[0xc4])
                                } else {
                                    Bytes::lit(&[0xc5])
                                }
                            } else {
                                let a1302 = {
                                    let a1300 = v0;
                                    let a1301 = 1u32;
                                    fixed_u32::bit_and(a1300, a1301)
                                };
                                let a1303 = 0u32;
                                if equal(a1302, a1303) {
                                    Bytes::lit(&[0xc6])
                                } else {
                                    Bytes::lit(&[0xc7])
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
                                    Bytes::lit(&[0xc8])
                                } else {
                                    Bytes::lit(&[0xc9])
                                }
                            } else {
                                let a1318 = {
                                    let a1316 = v0;
                                    let a1317 = 1u32;
                                    fixed_u32::bit_and(a1316, a1317)
                                };
                                let a1319 = 0u32;
                                if equal(a1318, a1319) {
                                    Bytes::lit(&[0xca])
                                } else {
                                    Bytes::lit(&[0xcb])
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
                                    Bytes::lit(&[0xcc])
                                } else {
                                    Bytes::lit(&[0xcd])
                                }
                            } else {
                                let a1330 = {
                                    let a1328 = v0;
                                    let a1329 = 1u32;
                                    fixed_u32::bit_and(a1328, a1329)
                                };
                                let a1331 = 0u32;
                                if equal(a1330, a1331) {
                                    Bytes::lit(&[0xce])
                                } else {
                                    Bytes::lit(&[0xcf])
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
                                    Bytes::lit(&[0xd0])
                                } else {
                                    Bytes::lit(&[0xd1])
                                }
                            } else {
                                let a1350 = {
                                    let a1348 = v0;
                                    let a1349 = 1u32;
                                    fixed_u32::bit_and(a1348, a1349)
                                };
                                let a1351 = 0u32;
                                if equal(a1350, a1351) {
                                    Bytes::lit(&[0xd2])
                                } else {
                                    Bytes::lit(&[0xd3])
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
                                    Bytes::lit(&[0xd4])
                                } else {
                                    Bytes::lit(&[0xd5])
                                }
                            } else {
                                let a1362 = {
                                    let a1360 = v0;
                                    let a1361 = 1u32;
                                    fixed_u32::bit_and(a1360, a1361)
                                };
                                let a1363 = 0u32;
                                if equal(a1362, a1363) {
                                    Bytes::lit(&[0xd6])
                                } else {
                                    Bytes::lit(&[0xd7])
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
                                    Bytes::lit(&[0xd8])
                                } else {
                                    Bytes::lit(&[0xd9])
                                }
                            } else {
                                let a1378 = {
                                    let a1376 = v0;
                                    let a1377 = 1u32;
                                    fixed_u32::bit_and(a1376, a1377)
                                };
                                let a1379 = 0u32;
                                if equal(a1378, a1379) {
                                    Bytes::lit(&[0xda])
                                } else {
                                    Bytes::lit(&[0xdb])
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
                                    Bytes::lit(&[0xdc])
                                } else {
                                    Bytes::lit(&[0xdd])
                                }
                            } else {
                                let a1390 = {
                                    let a1388 = v0;
                                    let a1389 = 1u32;
                                    fixed_u32::bit_and(a1388, a1389)
                                };
                                let a1391 = 0u32;
                                if equal(a1390, a1391) {
                                    Bytes::lit(&[0xde])
                                } else {
                                    Bytes::lit(&[0xdf])
                                }
                            }
                        }
                    }
                }
            } else {
                let a1394 = {
                    let a1392 = v0;
                    let a1393 = 16u32;
                    fixed_u32::bit_and(a1392, a1393)
                };
                let a1395 = 0u32;
                if equal(a1394, a1395) {
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
                                    Bytes::lit(&[0xe0])
                                } else {
                                    Bytes::lit(&[0xe1])
                                }
                            } else {
                                let a1414 = {
                                    let a1412 = v0;
                                    let a1413 = 1u32;
                                    fixed_u32::bit_and(a1412, a1413)
                                };
                                let a1415 = 0u32;
                                if equal(a1414, a1415) {
                                    Bytes::lit(&[0xe2])
                                } else {
                                    Bytes::lit(&[0xe3])
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
                                    Bytes::lit(&[0xe4])
                                } else {
                                    Bytes::lit(&[0xe5])
                                }
                            } else {
                                let a1426 = {
                                    let a1424 = v0;
                                    let a1425 = 1u32;
                                    fixed_u32::bit_and(a1424, a1425)
                                };
                                let a1427 = 0u32;
                                if equal(a1426, a1427) {
                                    Bytes::lit(&[0xe6])
                                } else {
                                    Bytes::lit(&[0xe7])
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
                                    Bytes::lit(&[0xe8])
                                } else {
                                    Bytes::lit(&[0xe9])
                                }
                            } else {
                                let a1442 = {
                                    let a1440 = v0;
                                    let a1441 = 1u32;
                                    fixed_u32::bit_and(a1440, a1441)
                                };
                                let a1443 = 0u32;
                                if equal(a1442, a1443) {
                                    Bytes::lit(&[0xea])
                                } else {
                                    Bytes::lit(&[0xeb])
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
                                    Bytes::lit(&[0xec])
                                } else {
                                    Bytes::lit(&[0xed])
                                }
                            } else {
                                let a1454 = {
                                    let a1452 = v0;
                                    let a1453 = 1u32;
                                    fixed_u32::bit_and(a1452, a1453)
                                };
                                let a1455 = 0u32;
                                if equal(a1454, a1455) {
                                    Bytes::lit(&[0xee])
                                } else {
                                    Bytes::lit(&[0xef])
                                }
                            }
                        }
                    }
                } else {
                    let a1458 = {
                        let a1456 = v0;
                        let a1457 = 8u32;
                        fixed_u32::bit_and(a1456, a1457)
                    };
                    let a1459 = 0u32;
                    if equal(a1458, a1459) {
                        let a1462 = {
                            let a1460 = v0;
                            let a1461 = 4u32;
                            fixed_u32::bit_and(a1460, a1461)
                        };
                        let a1463 = 0u32;
                        if equal(a1462, a1463) {
                            let a1466 = {
                                let a1464 = v0;
                                let a1465 = 2u32;
                                fixed_u32::bit_and(a1464, a1465)
                            };
                            let a1467 = 0u32;
                            if equal(a1466, a1467) {
                                let a1470 = {
                                    let a1468 = v0;
                                    let a1469 = 1u32;
                                    fixed_u32::bit_and(a1468, a1469)
                                };
                                let a1471 = 0u32;
                                if equal(a1470, a1471) {
                                    Bytes::lit(&[0xf0])
                                } else {
                                    Bytes::lit(&[0xf1])
                                }
                            } else {
                                let a1474 = {
                                    let a1472 = v0;
                                    let a1473 = 1u32;
                                    fixed_u32::bit_and(a1472, a1473)
                                };
                                let a1475 = 0u32;
                                if equal(a1474, a1475) {
                                    Bytes::lit(&[0xf2])
                                } else {
                                    Bytes::lit(&[0xf3])
                                }
                            }
                        } else {
                            let a1478 = {
                                let a1476 = v0;
                                let a1477 = 2u32;
                                fixed_u32::bit_and(a1476, a1477)
                            };
                            let a1479 = 0u32;
                            if equal(a1478, a1479) {
                                let a1482 = {
                                    let a1480 = v0;
                                    let a1481 = 1u32;
                                    fixed_u32::bit_and(a1480, a1481)
                                };
                                let a1483 = 0u32;
                                if equal(a1482, a1483) {
                                    Bytes::lit(&[0xf4])
                                } else {
                                    Bytes::lit(&[0xf5])
                                }
                            } else {
                                let a1486 = {
                                    let a1484 = v0;
                                    let a1485 = 1u32;
                                    fixed_u32::bit_and(a1484, a1485)
                                };
                                let a1487 = 0u32;
                                if equal(a1486, a1487) {
                                    Bytes::lit(&[0xf6])
                                } else {
                                    Bytes::lit(&[0xf7])
                                }
                            }
                        }
                    } else {
                        let a1490 = {
                            let a1488 = v0;
                            let a1489 = 4u32;
                            fixed_u32::bit_and(a1488, a1489)
                        };
                        let a1491 = 0u32;
                        if equal(a1490, a1491) {
                            let a1494 = {
                                let a1492 = v0;
                                let a1493 = 2u32;
                                fixed_u32::bit_and(a1492, a1493)
                            };
                            let a1495 = 0u32;
                            if equal(a1494, a1495) {
                                let a1498 = {
                                    let a1496 = v0;
                                    let a1497 = 1u32;
                                    fixed_u32::bit_and(a1496, a1497)
                                };
                                let a1499 = 0u32;
                                if equal(a1498, a1499) {
                                    Bytes::lit(&[0xf8])
                                } else {
                                    Bytes::lit(&[0xf9])
                                }
                            } else {
                                let a1502 = {
                                    let a1500 = v0;
                                    let a1501 = 1u32;
                                    fixed_u32::bit_and(a1500, a1501)
                                };
                                let a1503 = 0u32;
                                if equal(a1502, a1503) {
                                    Bytes::lit(&[0xfa])
                                } else {
                                    Bytes::lit(&[0xfb])
                                }
                            }
                        } else {
                            let a1506 = {
                                let a1504 = v0;
                                let a1505 = 2u32;
                                fixed_u32::bit_and(a1504, a1505)
                            };
                            let a1507 = 0u32;
                            if equal(a1506, a1507) {
                                let a1510 = {
                                    let a1508 = v0;
                                    let a1509 = 1u32;
                                    fixed_u32::bit_and(a1508, a1509)
                                };
                                let a1511 = 0u32;
                                if equal(a1510, a1511) {
                                    Bytes::lit(&[0xfc])
                                } else {
                                    Bytes::lit(&[0xfd])
                                }
                            } else {
                                let a1514 = {
                                    let a1512 = v0;
                                    let a1513 = 1u32;
                                    fixed_u32::bit_and(a1512, a1513)
                                };
                                let a1515 = 0u32;
                                if equal(a1514, a1515) {
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

pub fn f60(v0: Fn10, p1519: (Option<Bytes>, List<Bytes>), p1520: List<Bytes>) -> (Option<Bytes>, List<Bytes>) {
    let m1518 = (p1519, p1520);
    {
        let mut m1518 = m1518;
        loop {
            let step = {
                let (v1, v2) = m1518;
                let m1516 = v2;
                match m1516.uncons() {
                    None => {
                        Err::<((Option<Bytes>, List<Bytes>), List<Bytes>), (Option<Bytes>, List<Bytes>)>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<((Option<Bytes>, List<Bytes>), List<Bytes>), (Option<Bytes>, List<Bytes>)>(({
                            let c1517: Fn10 = v0.clone();
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

pub fn f61(v0: (Option<Bytes>, List<Bytes>), v1: Bytes) -> (Option<Bytes>, List<Bytes>) {
    let m1522 = {
        let (h1521, _) = v0;
        h1521
    };
    match m1522 {
        None => {
            (Some(v1), {
                let (_, h1526) = v0;
                h1526
            })
        }
        Some(v2) => {
            (None::<Bytes>, List::cons({
                let a1523 = v2;
                let a1524 = v1;
                append_bytes(a1523, a1524)
            }, {
                let (_, h1525) = v0;
                h1525
            }))
        }
    }
}

pub fn f62(v0: List<Bytes>, v1: Bytes) -> List<Bytes> {
    List::cons(v1, v0)
}

pub fn f63(v0: Bytes, v1: u64, v2: u64) -> R<u32> {
    Ok(f69(f25(v0.clone(), v1, v2), f25(v0.clone(), v1, {
        let a1527 = v2;
        let a1528 = 1u64;
        nat_add(a1527, a1528)?
    }), f25(v0.clone(), v1, {
        let a1529 = v2;
        let a1530 = 2u64;
        nat_add(a1529, a1530)?
    }), f25(v0, v1, {
        let a1531 = v2;
        let a1532 = 3u64;
        nat_add(a1531, a1532)?
    })))
}

pub fn f64(v0: Fn11, p1536: u64, p1537: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let m1535 = (p1536, p1537);
    {
        let mut m1535 = m1535;
        loop {
            let step = {
                let (v1, v2) = m1535;
                let m1533 = v1;
                if m1533 == 0 {
                    Err::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>(v2)
                } else {
                    let v3 = m1533 - 1;
                    Ok::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>((v3, {
                        let c1534: Fn11 = v0.clone();
                        c1534.apply(v2)
                    }))
                }
            };
            match step {
                Ok(next) => { m1535 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f65(v0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let v1: u64 = {
        let (h1538, _) = v0;
        h1538
    };
    let v2: u32 = {
        let (h1539, _) = {
            let (_, h1540) = v0;
            h1540
        };
        h1539
    };
    let v3: u32 = {
        let (_, h1541) = {
            let (_, h1542) = v0;
            h1542
        };
        h1541
    };
    ({
        let a1543 = v1;
        let a1544 = 2u64;
        let a1545 = 0u64;
        nat_quot(a1543, a1544, a1545)
    }, (f70(v2, 1u32), {
        let a1549 = {
            let a1546 = v1;
            let a1547 = 2u64;
            let a1548 = 0u64;
            nat_rem(a1546, a1547, a1548)
        };
        let a1550 = 1u64;
        if nat_eq(a1549, a1550) {
            let a1551 = v3;
            let a1552 = v2;
            fixed_u32::bit_or(a1551, a1552)
        } else {
            v3
        }
    }))
}

pub fn f66(v0: Adt9) -> Adt4 {
    f71(Fn12::F72, Adt4::C0(0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32), match v0 {
        Adt9::C0(_, _, _, _, _, _, h1553) => {
            h1553
        }
    })
}

pub fn f67(v0: Adt1, v1: Adt4, v2: u32, v3: u32, v4: u32, v5: u32) -> R<Adt9> {
    let v6: (u64, Adt12) = f73(Fn13::F74(v0.clone(), v1.clone()), 800u64, (0u64, Adt12::C0(f75(v0.clone(), v2, v3, v4, v5), Adt4::C0(0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32), List::<Adt10>::nil())))?;
    Ok(Adt9::C0(v0, v1, v2, v3, v4, v5, {
        let m1556 = {
            let (_, h1555) = v6;
            h1555
        };
        match m1556 {
            Adt12::C0(_, _, h1554) => {
                h1554
            }
        }
    }))
}

pub fn f68(v0: Adt1, v1: u32, v2: Adt8) -> R<Option<Adt8>> {
    let a1561 = {
        let a1558 = match v2.clone() {
            Adt8::C0(_, h1557, _, _) => {
                h1557
            }
        };
        let a1559 = 2u64;
        let a1560 = 0u64;
        nat_rem(a1558, a1559, a1560)
    };
    let a1562 = 0u64;
    if nat_eq(a1561, a1562) {
        let m1564 = match v2.clone() {
            Adt8::C0(_, _, h1563, _) => {
                h1563
            }
        };
        match m1564.uncons() {
            None => {
                Ok(None::<Adt8>)
            }
            Some((v3, v4)) => {
                let v5: Adt3 = f37(f38(v3.clone(), match v2.clone() {
                    Adt8::C0(h1565, _, _, _) => {
                        h1565
                    }
                }, v0, v1)?)?;
                Ok(Some(Adt8::C0(Adt5::C0(f39(match v5.clone() {
                    Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h1566) => {
                        h1566
                    }
                }), match v3.clone() {
                    Adt5::C0(_, h1567, _) => {
                        h1567
                    }
                }, {
                    let a1572 = match v3.clone() {
                        Adt5::C0(_, _, h1568) => {
                            h1568
                        }
                    };
                    let a1573 = {
                        let m1571 = match v2.clone() {
                            Adt8::C0(h1570, _, _, _) => {
                                h1570
                            }
                        };
                        match m1571 {
                            Adt5::C0(_, _, h1569) => {
                                h1569
                            }
                        }
                    };
                    nat_add(a1572, a1573)?
                }), {
                    let a1575 = match v2.clone() {
                        Adt8::C0(_, h1574, _, _) => {
                            h1574
                        }
                    };
                    let a1576 = 2u64;
                    let a1577 = 0u64;
                    nat_quot(a1575, a1576, a1577)
                }, v4, List::cons(Adt2::C0(v5), List::cons(Adt2::C2(v3, {
                    let a1579 = match v2.clone() {
                        Adt8::C0(_, _, h1578, _) => {
                            h1578
                        }
                    };
                    length_list(a1579)
                }), match v2 {
                    Adt8::C0(_, _, _, h1580) => {
                        h1580
                    }
                })))))
            }
        }
    } else {
        Ok(None::<Adt8>)
    }
}

pub fn f69(v0: u32, v1: u32, v2: u32, v3: u32) -> u32 {
    let a1585 = v0;
    let a1586 = {
        let a1583 = f70(v1, 8u32);
        let a1584 = {
            let a1581 = f70(v2, 16u32);
            let a1582 = f70(v3, 24u32);
            fixed_u32::bit_or(a1581, a1582)
        };
        fixed_u32::bit_or(a1583, a1584)
    };
    fixed_u32::bit_or(a1585, a1586)
}

pub fn f70(v0: u32, v1: u32) -> u32 {
    let m1589 = {
        let a1587 = v0;
        let a1588 = v1;
        fixed_u32::shift_left(a1587, a1588)
    };
    match m1589 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f71(v0: Fn12, p1593: Adt4, p1594: List<Adt10>) -> Adt4 {
    let m1592 = (p1593, p1594);
    {
        let mut m1592 = m1592;
        loop {
            let step = {
                let (v1, v2) = m1592;
                let m1590 = v2;
                match m1590.uncons() {
                    None => {
                        Err::<(Adt4, List<Adt10>), Adt4>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt4, List<Adt10>), Adt4>(({
                            let c1591: Fn12 = v0.clone();
                            c1591.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m1592 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f72(v0: Adt4, v1: Adt10) -> Adt4 {
    let a1596 = match v1.clone() {
        Adt10::C0(h1595, _, _, _, _, _, _, _, _) => {
            h1595
        }
    };
    let a1597 = 7u64;
    if nat_eq(a1596, a1597) {
        f76(v0, match v1.clone() {
            Adt10::C0(_, _, _, _, h1598, _, _, _, _) => {
                h1598
            }
        }, match v1 {
            Adt10::C0(_, _, _, _, _, _, _, _, h1599) => {
                h1599
            }
        })
    } else {
        v0
    }
}

pub fn f73(v0: Fn13, p1603: u64, p1604: (u64, Adt12)) -> R<(u64, Adt12)> {
    let m1602 = (p1603, p1604);
    {
        let mut m1602 = m1602;
        loop {
            let step = {
                let (v1, v2) = m1602;
                let m1600 = v1;
                if m1600 == 0 {
                    Err::<(u64, (u64, Adt12)), R<(u64, Adt12)>>(Ok(v2))
                } else {
                    let v3 = m1600 - 1;
                    Ok::<(u64, (u64, Adt12)), R<(u64, Adt12)>>((v3, {
                        let c1601: Fn13 = v0.clone();
                        c1601.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m1602 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f74(v0: Adt1, v1: Adt4, v2: (u64, Adt12)) -> R<(u64, Adt12)> {
    Ok(({
        let a1606 = {
            let (h1605, _) = v2.clone();
            h1605
        };
        let a1607 = 1u64;
        nat_add(a1606, a1607)?
    }, f77(v0, v1, {
        let (_, h1608) = v2;
        h1608
    }, {
        let (h1609, _) = v2;
        h1609
    })?))
}

pub fn f75(v0: Adt1, v1: u32, v2: u32, v3: u32, v4: u32) -> Adt4 {
    Adt4::C0(match v0.clone() {
        Adt1::C0(h1610, _, _, _, _, _, _, _) => {
            h1610
        }
    }, match v0.clone() {
        Adt1::C0(_, h1611, _, _, _, _, _, _) => {
            h1611
        }
    }, match v0.clone() {
        Adt1::C0(_, _, h1612, _, _, _, _, _) => {
            h1612
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, h1613, _, _, _, _) => {
            h1613
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, h1614, _, _, _) => {
            h1614
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, _, h1615, _, _) => {
            h1615
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, _, _, h1616, _) => {
            h1616
        }
    }, match v0 {
        Adt1::C0(_, _, _, _, _, _, _, h1617) => {
            h1617
        }
    }, 1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, v1, v2, v3, v4)
}

pub fn f76(v0: Adt4, v1: u64, v2: u32) -> Adt4 {
    let a1618 = v1;
    let a1619 = 8u64;
    if nat_lt(a1618, a1619) {
        let a1620 = v1;
        let a1621 = 4u64;
        if nat_lt(a1620, a1621) {
            let a1622 = v1;
            let a1623 = 2u64;
            if nat_lt(a1622, a1623) {
                let a1624 = v1;
                let a1625 = 1u64;
                if nat_lt(a1624, a1625) {
                    Adt4::C0(v2, match v0.clone() {
                        Adt4::C0(_, h1626, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1626
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1627, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1627
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1628, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1628
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1629, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1629
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1630, _, _, _, _, _, _, _, _, _, _) => {
                            h1630
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1631, _, _, _, _, _, _, _, _, _) => {
                            h1631
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1632, _, _, _, _, _, _, _, _) => {
                            h1632
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1633, _, _, _, _, _, _, _) => {
                            h1633
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1634, _, _, _, _, _, _) => {
                            h1634
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1635, _, _, _, _, _) => {
                            h1635
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1636, _, _, _, _) => {
                            h1636
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1637, _, _, _) => {
                            h1637
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1638, _, _) => {
                            h1638
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1639, _) => {
                            h1639
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1640) => {
                            h1640
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1641, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1641
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, h1642, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1642
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1643, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1643
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1644, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1644
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1645, _, _, _, _, _, _, _, _, _, _) => {
                            h1645
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1646, _, _, _, _, _, _, _, _, _) => {
                            h1646
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1647, _, _, _, _, _, _, _, _) => {
                            h1647
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1648, _, _, _, _, _, _, _) => {
                            h1648
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1649, _, _, _, _, _, _) => {
                            h1649
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1650, _, _, _, _, _) => {
                            h1650
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1651, _, _, _, _) => {
                            h1651
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1652, _, _, _) => {
                            h1652
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1653, _, _) => {
                            h1653
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1654, _) => {
                            h1654
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1655) => {
                            h1655
                        }
                    })
                }
            } else {
                let a1656 = v1;
                let a1657 = 3u64;
                if nat_lt(a1656, a1657) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1658, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1658
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1659, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1659
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, h1660, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1660
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1661, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1661
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1662, _, _, _, _, _, _, _, _, _, _) => {
                            h1662
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1663, _, _, _, _, _, _, _, _, _) => {
                            h1663
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1664, _, _, _, _, _, _, _, _) => {
                            h1664
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1665, _, _, _, _, _, _, _) => {
                            h1665
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1666, _, _, _, _, _, _) => {
                            h1666
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1667, _, _, _, _, _) => {
                            h1667
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1668, _, _, _, _) => {
                            h1668
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1669, _, _, _) => {
                            h1669
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1670, _, _) => {
                            h1670
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1671, _) => {
                            h1671
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1672) => {
                            h1672
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1673, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1673
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1674, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1674
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1675, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1675
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1676, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1676
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1677, _, _, _, _, _, _, _, _, _, _) => {
                            h1677
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1678, _, _, _, _, _, _, _, _, _) => {
                            h1678
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1679, _, _, _, _, _, _, _, _) => {
                            h1679
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1680, _, _, _, _, _, _, _) => {
                            h1680
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1681, _, _, _, _, _, _) => {
                            h1681
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1682, _, _, _, _, _) => {
                            h1682
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1683, _, _, _, _) => {
                            h1683
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1684, _, _, _) => {
                            h1684
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1685, _, _) => {
                            h1685
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1686, _) => {
                            h1686
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1687) => {
                            h1687
                        }
                    })
                }
            }
        } else {
            let a1688 = v1;
            let a1689 = 6u64;
            if nat_lt(a1688, a1689) {
                let a1690 = v1;
                let a1691 = 5u64;
                if nat_lt(a1690, a1691) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1692, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1692
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1693, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1693
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1694, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1694
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1695, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1695
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1696, _, _, _, _, _, _, _, _, _, _) => {
                            h1696
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1697, _, _, _, _, _, _, _, _, _) => {
                            h1697
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1698, _, _, _, _, _, _, _, _) => {
                            h1698
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1699, _, _, _, _, _, _, _) => {
                            h1699
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1700, _, _, _, _, _, _) => {
                            h1700
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1701, _, _, _, _, _) => {
                            h1701
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1702, _, _, _, _) => {
                            h1702
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1703, _, _, _) => {
                            h1703
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1704, _, _) => {
                            h1704
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1705, _) => {
                            h1705
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1706) => {
                            h1706
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1707, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1707
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1708, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1708
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1709, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1709
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1710, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1710
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1711, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1711
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1712, _, _, _, _, _, _, _, _, _) => {
                            h1712
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1713, _, _, _, _, _, _, _, _) => {
                            h1713
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1714, _, _, _, _, _, _, _) => {
                            h1714
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1715, _, _, _, _, _, _) => {
                            h1715
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1716, _, _, _, _, _) => {
                            h1716
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1717, _, _, _, _) => {
                            h1717
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1718, _, _, _) => {
                            h1718
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1719, _, _) => {
                            h1719
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1720, _) => {
                            h1720
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1721) => {
                            h1721
                        }
                    })
                }
            } else {
                let a1722 = v1;
                let a1723 = 7u64;
                if nat_lt(a1722, a1723) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1724, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1724
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1725, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1725
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1726, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1726
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1727, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1727
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1728, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1728
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1729, _, _, _, _, _, _, _, _, _, _) => {
                            h1729
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1730, _, _, _, _, _, _, _, _) => {
                            h1730
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1731, _, _, _, _, _, _, _) => {
                            h1731
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1732, _, _, _, _, _, _) => {
                            h1732
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1733, _, _, _, _, _) => {
                            h1733
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1734, _, _, _, _) => {
                            h1734
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1735, _, _, _) => {
                            h1735
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1736, _, _) => {
                            h1736
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1737, _) => {
                            h1737
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1738) => {
                            h1738
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1739, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1739
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1740, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1740
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1741, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1741
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1742, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1742
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1743, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1743
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1744, _, _, _, _, _, _, _, _, _, _) => {
                            h1744
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1745, _, _, _, _, _, _, _, _, _) => {
                            h1745
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1746, _, _, _, _, _, _, _) => {
                            h1746
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1747, _, _, _, _, _, _) => {
                            h1747
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1748, _, _, _, _, _) => {
                            h1748
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1749, _, _, _, _) => {
                            h1749
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1750, _, _, _) => {
                            h1750
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1751, _, _) => {
                            h1751
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1752, _) => {
                            h1752
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1753) => {
                            h1753
                        }
                    })
                }
            }
        }
    } else {
        let a1754 = v1;
        let a1755 = 12u64;
        if nat_lt(a1754, a1755) {
            let a1756 = v1;
            let a1757 = 10u64;
            if nat_lt(a1756, a1757) {
                let a1758 = v1;
                let a1759 = 9u64;
                if nat_lt(a1758, a1759) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1760, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1760
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1761, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1761
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1762, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1762
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1763, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1763
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1764, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1764
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1765, _, _, _, _, _, _, _, _, _, _) => {
                            h1765
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1766, _, _, _, _, _, _, _, _, _) => {
                            h1766
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1767, _, _, _, _, _, _, _, _) => {
                            h1767
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1768, _, _, _, _, _, _) => {
                            h1768
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1769, _, _, _, _, _) => {
                            h1769
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1770, _, _, _, _) => {
                            h1770
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1771, _, _, _) => {
                            h1771
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1772, _, _) => {
                            h1772
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1773, _) => {
                            h1773
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1774) => {
                            h1774
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1775, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1775
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1776, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1776
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1777, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1777
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1778, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1778
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1779, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1779
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1780, _, _, _, _, _, _, _, _, _, _) => {
                            h1780
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1781, _, _, _, _, _, _, _, _, _) => {
                            h1781
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1782, _, _, _, _, _, _, _, _) => {
                            h1782
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1783, _, _, _, _, _, _, _) => {
                            h1783
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1784, _, _, _, _, _) => {
                            h1784
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1785, _, _, _, _) => {
                            h1785
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1786, _, _, _) => {
                            h1786
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1787, _, _) => {
                            h1787
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1788, _) => {
                            h1788
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1789) => {
                            h1789
                        }
                    })
                }
            } else {
                let a1790 = v1;
                let a1791 = 11u64;
                if nat_lt(a1790, a1791) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1792, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1792
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1793, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1793
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1794, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1794
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1795, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1795
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1796, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1796
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1797, _, _, _, _, _, _, _, _, _, _) => {
                            h1797
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1798, _, _, _, _, _, _, _, _, _) => {
                            h1798
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1799, _, _, _, _, _, _, _, _) => {
                            h1799
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1800, _, _, _, _, _, _, _) => {
                            h1800
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1801, _, _, _, _, _, _) => {
                            h1801
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1802, _, _, _, _) => {
                            h1802
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1803, _, _, _) => {
                            h1803
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1804, _, _) => {
                            h1804
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1805, _) => {
                            h1805
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1806) => {
                            h1806
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1807, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1807
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1808, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1808
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1809, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1809
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1810, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1810
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1811, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1811
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1812, _, _, _, _, _, _, _, _, _, _) => {
                            h1812
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1813, _, _, _, _, _, _, _, _, _) => {
                            h1813
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1814, _, _, _, _, _, _, _, _) => {
                            h1814
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1815, _, _, _, _, _, _, _) => {
                            h1815
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1816, _, _, _, _, _, _) => {
                            h1816
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1817, _, _, _, _, _) => {
                            h1817
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1818, _, _, _) => {
                            h1818
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1819, _, _) => {
                            h1819
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1820, _) => {
                            h1820
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1821) => {
                            h1821
                        }
                    })
                }
            }
        } else {
            let a1822 = v1;
            let a1823 = 14u64;
            if nat_lt(a1822, a1823) {
                let a1824 = v1;
                let a1825 = 13u64;
                if nat_lt(a1824, a1825) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1826, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1826
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1827, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1827
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1828, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1828
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1829, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1829
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1830, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1830
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1831, _, _, _, _, _, _, _, _, _, _) => {
                            h1831
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1832, _, _, _, _, _, _, _, _, _) => {
                            h1832
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1833, _, _, _, _, _, _, _, _) => {
                            h1833
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1834, _, _, _, _, _, _, _) => {
                            h1834
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1835, _, _, _, _, _, _) => {
                            h1835
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1836, _, _, _, _, _) => {
                            h1836
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1837, _, _, _, _) => {
                            h1837
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1838, _, _) => {
                            h1838
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1839, _) => {
                            h1839
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1840) => {
                            h1840
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1841, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1841
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1842, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1842
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1843, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1843
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1844, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1844
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1845, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1845
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1846, _, _, _, _, _, _, _, _, _, _) => {
                            h1846
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1847, _, _, _, _, _, _, _, _, _) => {
                            h1847
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1848, _, _, _, _, _, _, _, _) => {
                            h1848
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1849, _, _, _, _, _, _, _) => {
                            h1849
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1850, _, _, _, _, _, _) => {
                            h1850
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1851, _, _, _, _, _) => {
                            h1851
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1852, _, _, _, _) => {
                            h1852
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1853, _, _, _) => {
                            h1853
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1854, _) => {
                            h1854
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1855) => {
                            h1855
                        }
                    })
                }
            } else {
                let a1856 = v1;
                let a1857 = 15u64;
                if nat_lt(a1856, a1857) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1858, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1858
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1859, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1859
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1860, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1860
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1861, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1861
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1862, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1862
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1863, _, _, _, _, _, _, _, _, _, _) => {
                            h1863
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1864, _, _, _, _, _, _, _, _, _) => {
                            h1864
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1865, _, _, _, _, _, _, _, _) => {
                            h1865
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1866, _, _, _, _, _, _, _) => {
                            h1866
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1867, _, _, _, _, _, _) => {
                            h1867
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1868, _, _, _, _, _) => {
                            h1868
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1869, _, _, _, _) => {
                            h1869
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1870, _, _, _) => {
                            h1870
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1871, _, _) => {
                            h1871
                        }
                    }, v2, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1872) => {
                            h1872
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1873, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1873
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1874, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1874
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1875, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1875
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1876, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1876
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1877, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1877
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1878, _, _, _, _, _, _, _, _, _, _) => {
                            h1878
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1879, _, _, _, _, _, _, _, _, _) => {
                            h1879
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1880, _, _, _, _, _, _, _, _) => {
                            h1880
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1881, _, _, _, _, _, _, _) => {
                            h1881
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1882, _, _, _, _, _, _) => {
                            h1882
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1883, _, _, _, _, _) => {
                            h1883
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1884, _, _, _, _) => {
                            h1884
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1885, _, _, _) => {
                            h1885
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1886, _, _) => {
                            h1886
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1887, _) => {
                            h1887
                        }
                    }, v2)
                }
            }
        }
    }
}

pub fn f77(v0: Adt1, v1: Adt4, v2: Adt12, v3: u64) -> R<Adt12> {
    let v4: u64 = {
        let a1888 = v3;
        let a1889 = 784u64;
        if nat_lt(a1888, a1889) {
            let a1890 = v3;
            let a1891 = 112u64;
            let a1892 = 0u64;
            nat_quot(a1890, a1891, a1892)
        } else {
            7u64
        }
    };
    let v5: Adt13 = f78({
        let a1893 = v3;
        let a1894 = 784u64;
        if nat_lt(a1893, a1894) {
            let a1895 = v3;
            let a1896 = 112u64;
            let a1897 = 0u64;
            nat_rem(a1895, a1896, a1897)
        } else {
            let a1898 = v3;
            let a1899 = 672u64;
            nat_sub(a1898, a1899)
        }
    });
    let v6: Adt4 = match v2.clone() {
        Adt12::C0(h1900, _, _) => {
            h1900
        }
    };
    let v7: u32 = f79(v6.clone(), match v5.clone() {
        Adt13::C0(_, _, _, h1901, _) => {
            h1901
        }
    });
    let v8: u32 = {
        let m1903 = match v5.clone() {
            Adt13::C0(_, _, h1902, _, _) => {
                h1902
            }
        };
        match m1903 {
            Adt11::C0 => {
                f79(v6.clone(), match v5.clone() {
                    Adt13::C0(_, _, _, _, h1904) => {
                        h1904
                    }
                })
            }
            Adt11::C1 => {
                f79(v1, f80(v4, match v5.clone() {
                    Adt13::C0(_, _, _, _, h1905) => {
                        h1905
                    }
                })?)
            }
            Adt11::C2 => {
                f79(v6.clone(), match v5.clone() {
                    Adt13::C0(_, _, _, _, h1906) => {
                        h1906
                    }
                })
            }
            Adt11::C3 => {
                f81(match v5.clone() {
                    Adt13::C0(_, _, _, _, h1907) => {
                        h1907
                    }
                })
            }
            Adt11::C4 => {
                f82(v0, match v5.clone() {
                    Adt13::C0(_, _, _, _, h1908) => {
                        h1908
                    }
                })
            }
        }
    };
    let v9: u32 = f83(match v5.clone() {
        Adt13::C0(_, _, h1909, _, _) => {
            h1909
        }
    }, v7, v8);
    Ok(Adt12::C0({
        let a1910 = v4;
        let a1911 = 7u64;
        if nat_eq(a1910, a1911) {
            v6
        } else {
            f76(v6, match v5.clone() {
                Adt13::C0(_, _, _, h1912, _) => {
                    h1912
                }
            }, v9)
        }
    }, {
        let a1913 = v4;
        let a1914 = 7u64;
        if nat_eq(a1913, a1914) {
            f76(match v2.clone() {
                Adt12::C0(_, h1915, _) => {
                    h1915
                }
            }, match v5.clone() {
                Adt13::C0(_, _, _, h1916, _) => {
                    h1916
                }
            }, v9)
        } else {
            match v2.clone() {
                Adt12::C0(_, h1917, _) => {
                    h1917
                }
            }
        }
    }, List::cons(Adt10::C0(v4, match v5.clone() {
        Adt13::C0(h1918, _, _, _, _) => {
            h1918
        }
    }, match v5.clone() {
        Adt13::C0(_, h1919, _, _, _) => {
            h1919
        }
    }, match v5.clone() {
        Adt13::C0(_, _, h1920, _, _) => {
            h1920
        }
    }, match v5.clone() {
        Adt13::C0(_, _, _, h1921, _) => {
            h1921
        }
    }, match v5 {
        Adt13::C0(_, _, _, _, h1922) => {
            h1922
        }
    }, v7, v8, v9), match v2 {
        Adt12::C0(_, _, h1923) => {
            h1923
        }
    })))
}

pub fn f78(v0: u64) -> Adt13 {
    let a1924 = v0;
    let a1925 = 64u64;
    if nat_lt(a1924, a1925) {
        let a1926 = v0;
        let a1927 = 32u64;
        if nat_lt(a1926, a1927) {
            let a1928 = v0;
            let a1929 = 16u64;
            if nat_lt(a1928, a1929) {
                let a1930 = v0;
                let a1931 = 8u64;
                if nat_lt(a1930, a1931) {
                    let a1932 = v0;
                    let a1933 = 4u64;
                    if nat_lt(a1932, a1933) {
                        let a1934 = v0;
                        let a1935 = 2u64;
                        if nat_lt(a1934, a1935) {
                            let a1936 = v0;
                            let a1937 = 1u64;
                            if nat_lt(a1936, a1937) {
                                Adt13::C0(0u64, 0u64, Adt11::C0, 0u64, 4u64)
                            } else {
                                Adt13::C0(0u64, 1u64, Adt11::C1, 0u64, 0u64)
                            }
                        } else {
                            let a1938 = v0;
                            let a1939 = 3u64;
                            if nat_lt(a1938, a1939) {
                                Adt13::C0(0u64, 2u64, Adt11::C2, 12u64, 0u64)
                            } else {
                                Adt13::C0(0u64, 3u64, Adt11::C3, 12u64, 16u64)
                            }
                        }
                    } else {
                        let a1940 = v0;
                        let a1941 = 6u64;
                        if nat_lt(a1940, a1941) {
                            let a1942 = v0;
                            let a1943 = 5u64;
                            if nat_lt(a1942, a1943) {
                                Adt13::C0(0u64, 4u64, Adt11::C0, 8u64, 12u64)
                            } else {
                                Adt13::C0(0u64, 5u64, Adt11::C2, 4u64, 8u64)
                            }
                        } else {
                            let a1944 = v0;
                            let a1945 = 7u64;
                            if nat_lt(a1944, a1945) {
                                Adt13::C0(0u64, 6u64, Adt11::C3, 4u64, 12u64)
                            } else {
                                Adt13::C0(0u64, 7u64, Adt11::C0, 0u64, 4u64)
                            }
                        }
                    }
                } else {
                    let a1946 = v0;
                    let a1947 = 12u64;
                    if nat_lt(a1946, a1947) {
                        let a1948 = v0;
                        let a1949 = 10u64;
                        if nat_lt(a1948, a1949) {
                            let a1950 = v0;
                            let a1951 = 9u64;
                            if nat_lt(a1950, a1951) {
                                Adt13::C0(0u64, 8u64, Adt11::C1, 0u64, 1u64)
                            } else {
                                Adt13::C0(0u64, 9u64, Adt11::C2, 12u64, 0u64)
                            }
                        } else {
                            let a1952 = v0;
                            let a1953 = 11u64;
                            if nat_lt(a1952, a1953) {
                                Adt13::C0(0u64, 10u64, Adt11::C3, 12u64, 8u64)
                            } else {
                                Adt13::C0(0u64, 11u64, Adt11::C0, 8u64, 12u64)
                            }
                        }
                    } else {
                        let a1954 = v0;
                        let a1955 = 14u64;
                        if nat_lt(a1954, a1955) {
                            let a1956 = v0;
                            let a1957 = 13u64;
                            if nat_lt(a1956, a1957) {
                                Adt13::C0(0u64, 12u64, Adt11::C2, 4u64, 8u64)
                            } else {
                                Adt13::C0(0u64, 13u64, Adt11::C3, 4u64, 7u64)
                            }
                        } else {
                            let a1958 = v0;
                            let a1959 = 15u64;
                            if nat_lt(a1958, a1959) {
                                Adt13::C0(1u64, 0u64, Adt11::C0, 1u64, 5u64)
                            } else {
                                Adt13::C0(1u64, 1u64, Adt11::C1, 1u64, 2u64)
                            }
                        }
                    }
                }
            } else {
                let a1960 = v0;
                let a1961 = 24u64;
                if nat_lt(a1960, a1961) {
                    let a1962 = v0;
                    let a1963 = 20u64;
                    if nat_lt(a1962, a1963) {
                        let a1964 = v0;
                        let a1965 = 18u64;
                        if nat_lt(a1964, a1965) {
                            let a1966 = v0;
                            let a1967 = 17u64;
                            if nat_lt(a1966, a1967) {
                                Adt13::C0(1u64, 2u64, Adt11::C2, 13u64, 1u64)
                            } else {
                                Adt13::C0(1u64, 3u64, Adt11::C3, 13u64, 16u64)
                            }
                        } else {
                            let a1968 = v0;
                            let a1969 = 19u64;
                            if nat_lt(a1968, a1969) {
                                Adt13::C0(1u64, 4u64, Adt11::C0, 9u64, 13u64)
                            } else {
                                Adt13::C0(1u64, 5u64, Adt11::C2, 5u64, 9u64)
                            }
                        }
                    } else {
                        let a1970 = v0;
                        let a1971 = 22u64;
                        if nat_lt(a1970, a1971) {
                            let a1972 = v0;
                            let a1973 = 21u64;
                            if nat_lt(a1972, a1973) {
                                Adt13::C0(1u64, 6u64, Adt11::C3, 5u64, 12u64)
                            } else {
                                Adt13::C0(1u64, 7u64, Adt11::C0, 1u64, 5u64)
                            }
                        } else {
                            let a1974 = v0;
                            let a1975 = 23u64;
                            if nat_lt(a1974, a1975) {
                                Adt13::C0(1u64, 8u64, Adt11::C1, 1u64, 3u64)
                            } else {
                                Adt13::C0(1u64, 9u64, Adt11::C2, 13u64, 1u64)
                            }
                        }
                    }
                } else {
                    let a1976 = v0;
                    let a1977 = 28u64;
                    if nat_lt(a1976, a1977) {
                        let a1978 = v0;
                        let a1979 = 26u64;
                        if nat_lt(a1978, a1979) {
                            let a1980 = v0;
                            let a1981 = 25u64;
                            if nat_lt(a1980, a1981) {
                                Adt13::C0(1u64, 10u64, Adt11::C3, 13u64, 8u64)
                            } else {
                                Adt13::C0(1u64, 11u64, Adt11::C0, 9u64, 13u64)
                            }
                        } else {
                            let a1982 = v0;
                            let a1983 = 27u64;
                            if nat_lt(a1982, a1983) {
                                Adt13::C0(1u64, 12u64, Adt11::C2, 5u64, 9u64)
                            } else {
                                Adt13::C0(1u64, 13u64, Adt11::C3, 5u64, 7u64)
                            }
                        }
                    } else {
                        let a1984 = v0;
                        let a1985 = 30u64;
                        if nat_lt(a1984, a1985) {
                            let a1986 = v0;
                            let a1987 = 29u64;
                            if nat_lt(a1986, a1987) {
                                Adt13::C0(2u64, 0u64, Adt11::C0, 2u64, 6u64)
                            } else {
                                Adt13::C0(2u64, 1u64, Adt11::C1, 2u64, 4u64)
                            }
                        } else {
                            let a1988 = v0;
                            let a1989 = 31u64;
                            if nat_lt(a1988, a1989) {
                                Adt13::C0(2u64, 2u64, Adt11::C2, 14u64, 2u64)
                            } else {
                                Adt13::C0(2u64, 3u64, Adt11::C3, 14u64, 16u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a1990 = v0;
            let a1991 = 48u64;
            if nat_lt(a1990, a1991) {
                let a1992 = v0;
                let a1993 = 40u64;
                if nat_lt(a1992, a1993) {
                    let a1994 = v0;
                    let a1995 = 36u64;
                    if nat_lt(a1994, a1995) {
                        let a1996 = v0;
                        let a1997 = 34u64;
                        if nat_lt(a1996, a1997) {
                            let a1998 = v0;
                            let a1999 = 33u64;
                            if nat_lt(a1998, a1999) {
                                Adt13::C0(2u64, 4u64, Adt11::C0, 10u64, 14u64)
                            } else {
                                Adt13::C0(2u64, 5u64, Adt11::C2, 6u64, 10u64)
                            }
                        } else {
                            let a2000 = v0;
                            let a2001 = 35u64;
                            if nat_lt(a2000, a2001) {
                                Adt13::C0(2u64, 6u64, Adt11::C3, 6u64, 12u64)
                            } else {
                                Adt13::C0(2u64, 7u64, Adt11::C0, 2u64, 6u64)
                            }
                        }
                    } else {
                        let a2002 = v0;
                        let a2003 = 38u64;
                        if nat_lt(a2002, a2003) {
                            let a2004 = v0;
                            let a2005 = 37u64;
                            if nat_lt(a2004, a2005) {
                                Adt13::C0(2u64, 8u64, Adt11::C1, 2u64, 5u64)
                            } else {
                                Adt13::C0(2u64, 9u64, Adt11::C2, 14u64, 2u64)
                            }
                        } else {
                            let a2006 = v0;
                            let a2007 = 39u64;
                            if nat_lt(a2006, a2007) {
                                Adt13::C0(2u64, 10u64, Adt11::C3, 14u64, 8u64)
                            } else {
                                Adt13::C0(2u64, 11u64, Adt11::C0, 10u64, 14u64)
                            }
                        }
                    }
                } else {
                    let a2008 = v0;
                    let a2009 = 44u64;
                    if nat_lt(a2008, a2009) {
                        let a2010 = v0;
                        let a2011 = 42u64;
                        if nat_lt(a2010, a2011) {
                            let a2012 = v0;
                            let a2013 = 41u64;
                            if nat_lt(a2012, a2013) {
                                Adt13::C0(2u64, 12u64, Adt11::C2, 6u64, 10u64)
                            } else {
                                Adt13::C0(2u64, 13u64, Adt11::C3, 6u64, 7u64)
                            }
                        } else {
                            let a2014 = v0;
                            let a2015 = 43u64;
                            if nat_lt(a2014, a2015) {
                                Adt13::C0(3u64, 0u64, Adt11::C0, 3u64, 7u64)
                            } else {
                                Adt13::C0(3u64, 1u64, Adt11::C1, 3u64, 6u64)
                            }
                        }
                    } else {
                        let a2016 = v0;
                        let a2017 = 46u64;
                        if nat_lt(a2016, a2017) {
                            let a2018 = v0;
                            let a2019 = 45u64;
                            if nat_lt(a2018, a2019) {
                                Adt13::C0(3u64, 2u64, Adt11::C2, 15u64, 3u64)
                            } else {
                                Adt13::C0(3u64, 3u64, Adt11::C3, 15u64, 16u64)
                            }
                        } else {
                            let a2020 = v0;
                            let a2021 = 47u64;
                            if nat_lt(a2020, a2021) {
                                Adt13::C0(3u64, 4u64, Adt11::C0, 11u64, 15u64)
                            } else {
                                Adt13::C0(3u64, 5u64, Adt11::C2, 7u64, 11u64)
                            }
                        }
                    }
                }
            } else {
                let a2022 = v0;
                let a2023 = 56u64;
                if nat_lt(a2022, a2023) {
                    let a2024 = v0;
                    let a2025 = 52u64;
                    if nat_lt(a2024, a2025) {
                        let a2026 = v0;
                        let a2027 = 50u64;
                        if nat_lt(a2026, a2027) {
                            let a2028 = v0;
                            let a2029 = 49u64;
                            if nat_lt(a2028, a2029) {
                                Adt13::C0(3u64, 6u64, Adt11::C3, 7u64, 12u64)
                            } else {
                                Adt13::C0(3u64, 7u64, Adt11::C0, 3u64, 7u64)
                            }
                        } else {
                            let a2030 = v0;
                            let a2031 = 51u64;
                            if nat_lt(a2030, a2031) {
                                Adt13::C0(3u64, 8u64, Adt11::C1, 3u64, 7u64)
                            } else {
                                Adt13::C0(3u64, 9u64, Adt11::C2, 15u64, 3u64)
                            }
                        }
                    } else {
                        let a2032 = v0;
                        let a2033 = 54u64;
                        if nat_lt(a2032, a2033) {
                            let a2034 = v0;
                            let a2035 = 53u64;
                            if nat_lt(a2034, a2035) {
                                Adt13::C0(3u64, 10u64, Adt11::C3, 15u64, 8u64)
                            } else {
                                Adt13::C0(3u64, 11u64, Adt11::C0, 11u64, 15u64)
                            }
                        } else {
                            let a2036 = v0;
                            let a2037 = 55u64;
                            if nat_lt(a2036, a2037) {
                                Adt13::C0(3u64, 12u64, Adt11::C2, 7u64, 11u64)
                            } else {
                                Adt13::C0(3u64, 13u64, Adt11::C3, 7u64, 7u64)
                            }
                        }
                    }
                } else {
                    let a2038 = v0;
                    let a2039 = 60u64;
                    if nat_lt(a2038, a2039) {
                        let a2040 = v0;
                        let a2041 = 58u64;
                        if nat_lt(a2040, a2041) {
                            let a2042 = v0;
                            let a2043 = 57u64;
                            if nat_lt(a2042, a2043) {
                                Adt13::C0(4u64, 0u64, Adt11::C0, 0u64, 5u64)
                            } else {
                                Adt13::C0(4u64, 1u64, Adt11::C1, 0u64, 8u64)
                            }
                        } else {
                            let a2044 = v0;
                            let a2045 = 59u64;
                            if nat_lt(a2044, a2045) {
                                Adt13::C0(4u64, 2u64, Adt11::C2, 15u64, 0u64)
                            } else {
                                Adt13::C0(4u64, 3u64, Adt11::C3, 15u64, 16u64)
                            }
                        }
                    } else {
                        let a2046 = v0;
                        let a2047 = 62u64;
                        if nat_lt(a2046, a2047) {
                            let a2048 = v0;
                            let a2049 = 61u64;
                            if nat_lt(a2048, a2049) {
                                Adt13::C0(4u64, 4u64, Adt11::C0, 10u64, 15u64)
                            } else {
                                Adt13::C0(4u64, 5u64, Adt11::C2, 5u64, 10u64)
                            }
                        } else {
                            let a2050 = v0;
                            let a2051 = 63u64;
                            if nat_lt(a2050, a2051) {
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
        let a2052 = v0;
        let a2053 = 96u64;
        if nat_lt(a2052, a2053) {
            let a2054 = v0;
            let a2055 = 80u64;
            if nat_lt(a2054, a2055) {
                let a2056 = v0;
                let a2057 = 72u64;
                if nat_lt(a2056, a2057) {
                    let a2058 = v0;
                    let a2059 = 68u64;
                    if nat_lt(a2058, a2059) {
                        let a2060 = v0;
                        let a2061 = 66u64;
                        if nat_lt(a2060, a2061) {
                            let a2062 = v0;
                            let a2063 = 65u64;
                            if nat_lt(a2062, a2063) {
                                Adt13::C0(4u64, 8u64, Adt11::C1, 0u64, 9u64)
                            } else {
                                Adt13::C0(4u64, 9u64, Adt11::C2, 15u64, 0u64)
                            }
                        } else {
                            let a2064 = v0;
                            let a2065 = 67u64;
                            if nat_lt(a2064, a2065) {
                                Adt13::C0(4u64, 10u64, Adt11::C3, 15u64, 8u64)
                            } else {
                                Adt13::C0(4u64, 11u64, Adt11::C0, 10u64, 15u64)
                            }
                        }
                    } else {
                        let a2066 = v0;
                        let a2067 = 70u64;
                        if nat_lt(a2066, a2067) {
                            let a2068 = v0;
                            let a2069 = 69u64;
                            if nat_lt(a2068, a2069) {
                                Adt13::C0(4u64, 12u64, Adt11::C2, 5u64, 10u64)
                            } else {
                                Adt13::C0(4u64, 13u64, Adt11::C3, 5u64, 7u64)
                            }
                        } else {
                            let a2070 = v0;
                            let a2071 = 71u64;
                            if nat_lt(a2070, a2071) {
                                Adt13::C0(5u64, 0u64, Adt11::C0, 1u64, 6u64)
                            } else {
                                Adt13::C0(5u64, 1u64, Adt11::C1, 1u64, 10u64)
                            }
                        }
                    }
                } else {
                    let a2072 = v0;
                    let a2073 = 76u64;
                    if nat_lt(a2072, a2073) {
                        let a2074 = v0;
                        let a2075 = 74u64;
                        if nat_lt(a2074, a2075) {
                            let a2076 = v0;
                            let a2077 = 73u64;
                            if nat_lt(a2076, a2077) {
                                Adt13::C0(5u64, 2u64, Adt11::C2, 12u64, 1u64)
                            } else {
                                Adt13::C0(5u64, 3u64, Adt11::C3, 12u64, 16u64)
                            }
                        } else {
                            let a2078 = v0;
                            let a2079 = 75u64;
                            if nat_lt(a2078, a2079) {
                                Adt13::C0(5u64, 4u64, Adt11::C0, 11u64, 12u64)
                            } else {
                                Adt13::C0(5u64, 5u64, Adt11::C2, 6u64, 11u64)
                            }
                        }
                    } else {
                        let a2080 = v0;
                        let a2081 = 78u64;
                        if nat_lt(a2080, a2081) {
                            let a2082 = v0;
                            let a2083 = 77u64;
                            if nat_lt(a2082, a2083) {
                                Adt13::C0(5u64, 6u64, Adt11::C3, 6u64, 12u64)
                            } else {
                                Adt13::C0(5u64, 7u64, Adt11::C0, 1u64, 6u64)
                            }
                        } else {
                            let a2084 = v0;
                            let a2085 = 79u64;
                            if nat_lt(a2084, a2085) {
                                Adt13::C0(5u64, 8u64, Adt11::C1, 1u64, 11u64)
                            } else {
                                Adt13::C0(5u64, 9u64, Adt11::C2, 12u64, 1u64)
                            }
                        }
                    }
                }
            } else {
                let a2086 = v0;
                let a2087 = 88u64;
                if nat_lt(a2086, a2087) {
                    let a2088 = v0;
                    let a2089 = 84u64;
                    if nat_lt(a2088, a2089) {
                        let a2090 = v0;
                        let a2091 = 82u64;
                        if nat_lt(a2090, a2091) {
                            let a2092 = v0;
                            let a2093 = 81u64;
                            if nat_lt(a2092, a2093) {
                                Adt13::C0(5u64, 10u64, Adt11::C3, 12u64, 8u64)
                            } else {
                                Adt13::C0(5u64, 11u64, Adt11::C0, 11u64, 12u64)
                            }
                        } else {
                            let a2094 = v0;
                            let a2095 = 83u64;
                            if nat_lt(a2094, a2095) {
                                Adt13::C0(5u64, 12u64, Adt11::C2, 6u64, 11u64)
                            } else {
                                Adt13::C0(5u64, 13u64, Adt11::C3, 6u64, 7u64)
                            }
                        }
                    } else {
                        let a2096 = v0;
                        let a2097 = 86u64;
                        if nat_lt(a2096, a2097) {
                            let a2098 = v0;
                            let a2099 = 85u64;
                            if nat_lt(a2098, a2099) {
                                Adt13::C0(6u64, 0u64, Adt11::C0, 2u64, 7u64)
                            } else {
                                Adt13::C0(6u64, 1u64, Adt11::C1, 2u64, 12u64)
                            }
                        } else {
                            let a2100 = v0;
                            let a2101 = 87u64;
                            if nat_lt(a2100, a2101) {
                                Adt13::C0(6u64, 2u64, Adt11::C2, 13u64, 2u64)
                            } else {
                                Adt13::C0(6u64, 3u64, Adt11::C3, 13u64, 16u64)
                            }
                        }
                    }
                } else {
                    let a2102 = v0;
                    let a2103 = 92u64;
                    if nat_lt(a2102, a2103) {
                        let a2104 = v0;
                        let a2105 = 90u64;
                        if nat_lt(a2104, a2105) {
                            let a2106 = v0;
                            let a2107 = 89u64;
                            if nat_lt(a2106, a2107) {
                                Adt13::C0(6u64, 4u64, Adt11::C0, 8u64, 13u64)
                            } else {
                                Adt13::C0(6u64, 5u64, Adt11::C2, 7u64, 8u64)
                            }
                        } else {
                            let a2108 = v0;
                            let a2109 = 91u64;
                            if nat_lt(a2108, a2109) {
                                Adt13::C0(6u64, 6u64, Adt11::C3, 7u64, 12u64)
                            } else {
                                Adt13::C0(6u64, 7u64, Adt11::C0, 2u64, 7u64)
                            }
                        }
                    } else {
                        let a2110 = v0;
                        let a2111 = 94u64;
                        if nat_lt(a2110, a2111) {
                            let a2112 = v0;
                            let a2113 = 93u64;
                            if nat_lt(a2112, a2113) {
                                Adt13::C0(6u64, 8u64, Adt11::C1, 2u64, 13u64)
                            } else {
                                Adt13::C0(6u64, 9u64, Adt11::C2, 13u64, 2u64)
                            }
                        } else {
                            let a2114 = v0;
                            let a2115 = 95u64;
                            if nat_lt(a2114, a2115) {
                                Adt13::C0(6u64, 10u64, Adt11::C3, 13u64, 8u64)
                            } else {
                                Adt13::C0(6u64, 11u64, Adt11::C0, 8u64, 13u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a2116 = v0;
            let a2117 = 112u64;
            if nat_lt(a2116, a2117) {
                let a2118 = v0;
                let a2119 = 104u64;
                if nat_lt(a2118, a2119) {
                    let a2120 = v0;
                    let a2121 = 100u64;
                    if nat_lt(a2120, a2121) {
                        let a2122 = v0;
                        let a2123 = 98u64;
                        if nat_lt(a2122, a2123) {
                            let a2124 = v0;
                            let a2125 = 97u64;
                            if nat_lt(a2124, a2125) {
                                Adt13::C0(6u64, 12u64, Adt11::C2, 7u64, 8u64)
                            } else {
                                Adt13::C0(6u64, 13u64, Adt11::C3, 7u64, 7u64)
                            }
                        } else {
                            let a2126 = v0;
                            let a2127 = 99u64;
                            if nat_lt(a2126, a2127) {
                                Adt13::C0(7u64, 0u64, Adt11::C0, 3u64, 4u64)
                            } else {
                                Adt13::C0(7u64, 1u64, Adt11::C1, 3u64, 14u64)
                            }
                        }
                    } else {
                        let a2128 = v0;
                        let a2129 = 102u64;
                        if nat_lt(a2128, a2129) {
                            let a2130 = v0;
                            let a2131 = 101u64;
                            if nat_lt(a2130, a2131) {
                                Adt13::C0(7u64, 2u64, Adt11::C2, 14u64, 3u64)
                            } else {
                                Adt13::C0(7u64, 3u64, Adt11::C3, 14u64, 16u64)
                            }
                        } else {
                            let a2132 = v0;
                            let a2133 = 103u64;
                            if nat_lt(a2132, a2133) {
                                Adt13::C0(7u64, 4u64, Adt11::C0, 9u64, 14u64)
                            } else {
                                Adt13::C0(7u64, 5u64, Adt11::C2, 4u64, 9u64)
                            }
                        }
                    }
                } else {
                    let a2134 = v0;
                    let a2135 = 108u64;
                    if nat_lt(a2134, a2135) {
                        let a2136 = v0;
                        let a2137 = 106u64;
                        if nat_lt(a2136, a2137) {
                            let a2138 = v0;
                            let a2139 = 105u64;
                            if nat_lt(a2138, a2139) {
                                Adt13::C0(7u64, 6u64, Adt11::C3, 4u64, 12u64)
                            } else {
                                Adt13::C0(7u64, 7u64, Adt11::C0, 3u64, 4u64)
                            }
                        } else {
                            let a2140 = v0;
                            let a2141 = 107u64;
                            if nat_lt(a2140, a2141) {
                                Adt13::C0(7u64, 8u64, Adt11::C1, 3u64, 15u64)
                            } else {
                                Adt13::C0(7u64, 9u64, Adt11::C2, 14u64, 3u64)
                            }
                        }
                    } else {
                        let a2142 = v0;
                        let a2143 = 110u64;
                        if nat_lt(a2142, a2143) {
                            let a2144 = v0;
                            let a2145 = 109u64;
                            if nat_lt(a2144, a2145) {
                                Adt13::C0(7u64, 10u64, Adt11::C3, 14u64, 8u64)
                            } else {
                                Adt13::C0(7u64, 11u64, Adt11::C0, 9u64, 14u64)
                            }
                        } else {
                            let a2146 = v0;
                            let a2147 = 111u64;
                            if nat_lt(a2146, a2147) {
                                Adt13::C0(7u64, 12u64, Adt11::C2, 4u64, 9u64)
                            } else {
                                Adt13::C0(7u64, 13u64, Adt11::C3, 4u64, 7u64)
                            }
                        }
                    }
                }
            } else {
                let a2148 = v0;
                let a2149 = 120u64;
                if nat_lt(a2148, a2149) {
                    let a2150 = v0;
                    let a2151 = 116u64;
                    if nat_lt(a2150, a2151) {
                        let a2152 = v0;
                        let a2153 = 114u64;
                        if nat_lt(a2152, a2153) {
                            let a2154 = v0;
                            let a2155 = 113u64;
                            if nat_lt(a2154, a2155) {
                                Adt13::C0(0u64, 0u64, Adt11::C2, 0u64, 8u64)
                            } else {
                                Adt13::C0(0u64, 1u64, Adt11::C2, 1u64, 9u64)
                            }
                        } else {
                            let a2156 = v0;
                            let a2157 = 115u64;
                            if nat_lt(a2156, a2157) {
                                Adt13::C0(0u64, 2u64, Adt11::C2, 2u64, 10u64)
                            } else {
                                Adt13::C0(0u64, 3u64, Adt11::C2, 3u64, 11u64)
                            }
                        }
                    } else {
                        let a2158 = v0;
                        let a2159 = 118u64;
                        if nat_lt(a2158, a2159) {
                            let a2160 = v0;
                            let a2161 = 117u64;
                            if nat_lt(a2160, a2161) {
                                Adt13::C0(0u64, 4u64, Adt11::C2, 4u64, 12u64)
                            } else {
                                Adt13::C0(0u64, 5u64, Adt11::C2, 5u64, 13u64)
                            }
                        } else {
                            let a2162 = v0;
                            let a2163 = 119u64;
                            if nat_lt(a2162, a2163) {
                                Adt13::C0(0u64, 6u64, Adt11::C2, 6u64, 14u64)
                            } else {
                                Adt13::C0(0u64, 7u64, Adt11::C2, 7u64, 15u64)
                            }
                        }
                    }
                } else {
                    let a2164 = v0;
                    let a2165 = 124u64;
                    if nat_lt(a2164, a2165) {
                        let a2166 = v0;
                        let a2167 = 122u64;
                        if nat_lt(a2166, a2167) {
                            let a2168 = v0;
                            let a2169 = 121u64;
                            if nat_lt(a2168, a2169) {
                                Adt13::C0(0u64, 8u64, Adt11::C4, 8u64, 0u64)
                            } else {
                                Adt13::C0(0u64, 9u64, Adt11::C4, 9u64, 1u64)
                            }
                        } else {
                            let a2170 = v0;
                            let a2171 = 123u64;
                            if nat_lt(a2170, a2171) {
                                Adt13::C0(0u64, 10u64, Adt11::C4, 10u64, 2u64)
                            } else {
                                Adt13::C0(0u64, 11u64, Adt11::C4, 11u64, 3u64)
                            }
                        }
                    } else {
                        let a2172 = v0;
                        let a2173 = 126u64;
                        if nat_lt(a2172, a2173) {
                            let a2174 = v0;
                            let a2175 = 125u64;
                            if nat_lt(a2174, a2175) {
                                Adt13::C0(0u64, 12u64, Adt11::C4, 12u64, 4u64)
                            } else {
                                Adt13::C0(0u64, 13u64, Adt11::C4, 13u64, 5u64)
                            }
                        } else {
                            let a2176 = v0;
                            let a2177 = 127u64;
                            if nat_lt(a2176, a2177) {
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

pub fn f79(v0: Adt4, v1: u64) -> u32 {
    let a2178 = v1;
    let a2179 = 8u64;
    if nat_lt(a2178, a2179) {
        let a2180 = v1;
        let a2181 = 4u64;
        if nat_lt(a2180, a2181) {
            let a2182 = v1;
            let a2183 = 2u64;
            if nat_lt(a2182, a2183) {
                let a2184 = v1;
                let a2185 = 1u64;
                if nat_lt(a2184, a2185) {
                    match v0 {
                        Adt4::C0(h2186, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2186
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, h2187, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2187
                        }
                    }
                }
            } else {
                let a2188 = v1;
                let a2189 = 3u64;
                if nat_lt(a2188, a2189) {
                    match v0 {
                        Adt4::C0(_, _, h2190, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2190
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, h2191, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2191
                        }
                    }
                }
            }
        } else {
            let a2192 = v1;
            let a2193 = 6u64;
            if nat_lt(a2192, a2193) {
                let a2194 = v1;
                let a2195 = 5u64;
                if nat_lt(a2194, a2195) {
                    match v0 {
                        Adt4::C0(_, _, _, _, h2196, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2196
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, h2197, _, _, _, _, _, _, _, _, _, _) => {
                            h2197
                        }
                    }
                }
            } else {
                let a2198 = v1;
                let a2199 = 7u64;
                if nat_lt(a2198, a2199) {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, h2200, _, _, _, _, _, _, _, _, _) => {
                            h2200
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, h2201, _, _, _, _, _, _, _, _) => {
                            h2201
                        }
                    }
                }
            }
        }
    } else {
        let a2202 = v1;
        let a2203 = 12u64;
        if nat_lt(a2202, a2203) {
            let a2204 = v1;
            let a2205 = 10u64;
            if nat_lt(a2204, a2205) {
                let a2206 = v1;
                let a2207 = 9u64;
                if nat_lt(a2206, a2207) {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, h2208, _, _, _, _, _, _, _) => {
                            h2208
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h2209, _, _, _, _, _, _) => {
                            h2209
                        }
                    }
                }
            } else {
                let a2210 = v1;
                let a2211 = 11u64;
                if nat_lt(a2210, a2211) {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h2212, _, _, _, _, _) => {
                            h2212
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h2213, _, _, _, _) => {
                            h2213
                        }
                    }
                }
            }
        } else {
            let a2214 = v1;
            let a2215 = 14u64;
            if nat_lt(a2214, a2215) {
                let a2216 = v1;
                let a2217 = 13u64;
                if nat_lt(a2216, a2217) {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2218, _, _, _) => {
                            h2218
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2219, _, _) => {
                            h2219
                        }
                    }
                }
            } else {
                let a2220 = v1;
                let a2221 = 15u64;
                if nat_lt(a2220, a2221) {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2222, _) => {
                            h2222
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2223) => {
                            h2223
                        }
                    }
                }
            }
        }
    }
}

pub fn f80(v0: u64, v1: u64) -> R<u64> {
    let v2: u64 = {
        let a2226 = {
            let a2224 = v0;
            let a2225 = 16u64;
            nat_mul(a2224, a2225)?
        };
        let a2227 = v1;
        nat_add(a2226, a2227)?
    };
    let a2228 = v2;
    let a2229 = 56u64;
    if nat_lt(a2228, a2229) {
        let a2230 = v2;
        let a2231 = 28u64;
        if nat_lt(a2230, a2231) {
            let a2232 = v2;
            let a2233 = 14u64;
            if nat_lt(a2232, a2233) {
                let a2234 = v2;
                let a2235 = 7u64;
                if nat_lt(a2234, a2235) {
                    let a2236 = v2;
                    let a2237 = 3u64;
                    if nat_lt(a2236, a2237) {
                        let a2238 = v2;
                        let a2239 = 1u64;
                        if nat_lt(a2238, a2239) {
                            Ok(0u64)
                        } else {
                            let a2240 = v2;
                            let a2241 = 2u64;
                            if nat_lt(a2240, a2241) {
                                Ok(1u64)
                            } else {
                                Ok(2u64)
                            }
                        }
                    } else {
                        let a2242 = v2;
                        let a2243 = 5u64;
                        if nat_lt(a2242, a2243) {
                            let a2244 = v2;
                            let a2245 = 4u64;
                            if nat_lt(a2244, a2245) {
                                Ok(3u64)
                            } else {
                                Ok(4u64)
                            }
                        } else {
                            let a2246 = v2;
                            let a2247 = 6u64;
                            if nat_lt(a2246, a2247) {
                                Ok(5u64)
                            } else {
                                Ok(6u64)
                            }
                        }
                    }
                } else {
                    let a2248 = v2;
                    let a2249 = 10u64;
                    if nat_lt(a2248, a2249) {
                        let a2250 = v2;
                        let a2251 = 8u64;
                        if nat_lt(a2250, a2251) {
                            Ok(7u64)
                        } else {
                            let a2252 = v2;
                            let a2253 = 9u64;
                            if nat_lt(a2252, a2253) {
                                Ok(8u64)
                            } else {
                                Ok(9u64)
                            }
                        }
                    } else {
                        let a2254 = v2;
                        let a2255 = 12u64;
                        if nat_lt(a2254, a2255) {
                            let a2256 = v2;
                            let a2257 = 11u64;
                            if nat_lt(a2256, a2257) {
                                Ok(10u64)
                            } else {
                                Ok(11u64)
                            }
                        } else {
                            let a2258 = v2;
                            let a2259 = 13u64;
                            if nat_lt(a2258, a2259) {
                                Ok(12u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    }
                }
            } else {
                let a2260 = v2;
                let a2261 = 21u64;
                if nat_lt(a2260, a2261) {
                    let a2262 = v2;
                    let a2263 = 17u64;
                    if nat_lt(a2262, a2263) {
                        let a2264 = v2;
                        let a2265 = 15u64;
                        if nat_lt(a2264, a2265) {
                            Ok(14u64)
                        } else {
                            let a2266 = v2;
                            let a2267 = 16u64;
                            if nat_lt(a2266, a2267) {
                                Ok(15u64)
                            } else {
                                Ok(2u64)
                            }
                        }
                    } else {
                        let a2268 = v2;
                        let a2269 = 19u64;
                        if nat_lt(a2268, a2269) {
                            let a2270 = v2;
                            let a2271 = 18u64;
                            if nat_lt(a2270, a2271) {
                                Ok(6u64)
                            } else {
                                Ok(3u64)
                            }
                        } else {
                            let a2272 = v2;
                            let a2273 = 20u64;
                            if nat_lt(a2272, a2273) {
                                Ok(10u64)
                            } else {
                                Ok(7u64)
                            }
                        }
                    }
                } else {
                    let a2274 = v2;
                    let a2275 = 24u64;
                    if nat_lt(a2274, a2275) {
                        let a2276 = v2;
                        let a2277 = 22u64;
                        if nat_lt(a2276, a2277) {
                            Ok(0u64)
                        } else {
                            let a2278 = v2;
                            let a2279 = 23u64;
                            if nat_lt(a2278, a2279) {
                                Ok(4u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    } else {
                        let a2280 = v2;
                        let a2281 = 26u64;
                        if nat_lt(a2280, a2281) {
                            let a2282 = v2;
                            let a2283 = 25u64;
                            if nat_lt(a2282, a2283) {
                                Ok(1u64)
                            } else {
                                Ok(11u64)
                            }
                        } else {
                            let a2284 = v2;
                            let a2285 = 27u64;
                            if nat_lt(a2284, a2285) {
                                Ok(12u64)
                            } else {
                                Ok(5u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a2286 = v2;
            let a2287 = 42u64;
            if nat_lt(a2286, a2287) {
                let a2288 = v2;
                let a2289 = 35u64;
                if nat_lt(a2288, a2289) {
                    let a2290 = v2;
                    let a2291 = 31u64;
                    if nat_lt(a2290, a2291) {
                        let a2292 = v2;
                        let a2293 = 29u64;
                        if nat_lt(a2292, a2293) {
                            Ok(9u64)
                        } else {
                            let a2294 = v2;
                            let a2295 = 30u64;
                            if nat_lt(a2294, a2295) {
                                Ok(14u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    } else {
                        let a2296 = v2;
                        let a2297 = 33u64;
                        if nat_lt(a2296, a2297) {
                            let a2298 = v2;
                            let a2299 = 32u64;
                            if nat_lt(a2298, a2299) {
                                Ok(8u64)
                            } else {
                                Ok(3u64)
                            }
                        } else {
                            let a2300 = v2;
                            let a2301 = 34u64;
                            if nat_lt(a2300, a2301) {
                                Ok(4u64)
                            } else {
                                Ok(10u64)
                            }
                        }
                    }
                } else {
                    let a2302 = v2;
                    let a2303 = 38u64;
                    if nat_lt(a2302, a2303) {
                        let a2304 = v2;
                        let a2305 = 36u64;
                        if nat_lt(a2304, a2305) {
                            Ok(12u64)
                        } else {
                            let a2306 = v2;
                            let a2307 = 37u64;
                            if nat_lt(a2306, a2307) {
                                Ok(13u64)
                            } else {
                                Ok(2u64)
                            }
                        }
                    } else {
                        let a2308 = v2;
                        let a2309 = 40u64;
                        if nat_lt(a2308, a2309) {
                            let a2310 = v2;
                            let a2311 = 39u64;
                            if nat_lt(a2310, a2311) {
                                Ok(7u64)
                            } else {
                                Ok(14u64)
                            }
                        } else {
                            let a2312 = v2;
                            let a2313 = 41u64;
                            if nat_lt(a2312, a2313) {
                                Ok(6u64)
                            } else {
                                Ok(5u64)
                            }
                        }
                    }
                }
            } else {
                let a2314 = v2;
                let a2315 = 49u64;
                if nat_lt(a2314, a2315) {
                    let a2316 = v2;
                    let a2317 = 45u64;
                    if nat_lt(a2316, a2317) {
                        let a2318 = v2;
                        let a2319 = 43u64;
                        if nat_lt(a2318, a2319) {
                            Ok(9u64)
                        } else {
                            let a2320 = v2;
                            let a2321 = 44u64;
                            if nat_lt(a2320, a2321) {
                                Ok(0u64)
                            } else {
                                Ok(11u64)
                            }
                        }
                    } else {
                        let a2322 = v2;
                        let a2323 = 47u64;
                        if nat_lt(a2322, a2323) {
                            let a2324 = v2;
                            let a2325 = 46u64;
                            if nat_lt(a2324, a2325) {
                                Ok(15u64)
                            } else {
                                Ok(8u64)
                            }
                        } else {
                            let a2326 = v2;
                            let a2327 = 48u64;
                            if nat_lt(a2326, a2327) {
                                Ok(1u64)
                            } else {
                                Ok(10u64)
                            }
                        }
                    }
                } else {
                    let a2328 = v2;
                    let a2329 = 52u64;
                    if nat_lt(a2328, a2329) {
                        let a2330 = v2;
                        let a2331 = 50u64;
                        if nat_lt(a2330, a2331) {
                            Ok(7u64)
                        } else {
                            let a2332 = v2;
                            let a2333 = 51u64;
                            if nat_lt(a2332, a2333) {
                                Ok(12u64)
                            } else {
                                Ok(9u64)
                            }
                        }
                    } else {
                        let a2334 = v2;
                        let a2335 = 54u64;
                        if nat_lt(a2334, a2335) {
                            let a2336 = v2;
                            let a2337 = 53u64;
                            if nat_lt(a2336, a2337) {
                                Ok(14u64)
                            } else {
                                Ok(3u64)
                            }
                        } else {
                            let a2338 = v2;
                            let a2339 = 55u64;
                            if nat_lt(a2338, a2339) {
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
        let a2340 = v2;
        let a2341 = 84u64;
        if nat_lt(a2340, a2341) {
            let a2342 = v2;
            let a2343 = 70u64;
            if nat_lt(a2342, a2343) {
                let a2344 = v2;
                let a2345 = 63u64;
                if nat_lt(a2344, a2345) {
                    let a2346 = v2;
                    let a2347 = 59u64;
                    if nat_lt(a2346, a2347) {
                        let a2348 = v2;
                        let a2349 = 57u64;
                        if nat_lt(a2348, a2349) {
                            Ok(4u64)
                        } else {
                            let a2350 = v2;
                            let a2351 = 58u64;
                            if nat_lt(a2350, a2351) {
                                Ok(0u64)
                            } else {
                                Ok(11u64)
                            }
                        }
                    } else {
                        let a2352 = v2;
                        let a2353 = 61u64;
                        if nat_lt(a2352, a2353) {
                            let a2354 = v2;
                            let a2355 = 60u64;
                            if nat_lt(a2354, a2355) {
                                Ok(2u64)
                            } else {
                                Ok(5u64)
                            }
                        } else {
                            let a2356 = v2;
                            let a2357 = 62u64;
                            if nat_lt(a2356, a2357) {
                                Ok(8u64)
                            } else {
                                Ok(1u64)
                            }
                        }
                    }
                } else {
                    let a2358 = v2;
                    let a2359 = 66u64;
                    if nat_lt(a2358, a2359) {
                        let a2360 = v2;
                        let a2361 = 64u64;
                        if nat_lt(a2360, a2361) {
                            Ok(6u64)
                        } else {
                            let a2362 = v2;
                            let a2363 = 65u64;
                            if nat_lt(a2362, a2363) {
                                Ok(12u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    } else {
                        let a2364 = v2;
                        let a2365 = 68u64;
                        if nat_lt(a2364, a2365) {
                            let a2366 = v2;
                            let a2367 = 67u64;
                            if nat_lt(a2366, a2367) {
                                Ok(9u64)
                            } else {
                                Ok(11u64)
                            }
                        } else {
                            let a2368 = v2;
                            let a2369 = 69u64;
                            if nat_lt(a2368, a2369) {
                                Ok(15u64)
                            } else {
                                Ok(10u64)
                            }
                        }
                    }
                }
            } else {
                let a2370 = v2;
                let a2371 = 77u64;
                if nat_lt(a2370, a2371) {
                    let a2372 = v2;
                    let a2373 = 73u64;
                    if nat_lt(a2372, a2373) {
                        let a2374 = v2;
                        let a2375 = 71u64;
                        if nat_lt(a2374, a2375) {
                            Ok(14u64)
                        } else {
                            let a2376 = v2;
                            let a2377 = 72u64;
                            if nat_lt(a2376, a2377) {
                                Ok(8u64)
                            } else {
                                Ok(7u64)
                            }
                        }
                    } else {
                        let a2378 = v2;
                        let a2379 = 75u64;
                        if nat_lt(a2378, a2379) {
                            let a2380 = v2;
                            let a2381 = 74u64;
                            if nat_lt(a2380, a2381) {
                                Ok(2u64)
                            } else {
                                Ok(5u64)
                            }
                        } else {
                            let a2382 = v2;
                            let a2383 = 76u64;
                            if nat_lt(a2382, a2383) {
                                Ok(3u64)
                            } else {
                                Ok(0u64)
                            }
                        }
                    }
                } else {
                    let a2384 = v2;
                    let a2385 = 80u64;
                    if nat_lt(a2384, a2385) {
                        let a2386 = v2;
                        let a2387 = 78u64;
                        if nat_lt(a2386, a2387) {
                            Ok(1u64)
                        } else {
                            let a2388 = v2;
                            let a2389 = 79u64;
                            if nat_lt(a2388, a2389) {
                                Ok(6u64)
                            } else {
                                Ok(4u64)
                            }
                        }
                    } else {
                        let a2390 = v2;
                        let a2391 = 82u64;
                        if nat_lt(a2390, a2391) {
                            let a2392 = v2;
                            let a2393 = 81u64;
                            if nat_lt(a2392, a2393) {
                                Ok(9u64)
                            } else {
                                Ok(14u64)
                            }
                        } else {
                            let a2394 = v2;
                            let a2395 = 83u64;
                            if nat_lt(a2394, a2395) {
                                Ok(11u64)
                            } else {
                                Ok(5u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a2396 = v2;
            let a2397 = 98u64;
            if nat_lt(a2396, a2397) {
                let a2398 = v2;
                let a2399 = 91u64;
                if nat_lt(a2398, a2399) {
                    let a2400 = v2;
                    let a2401 = 87u64;
                    if nat_lt(a2400, a2401) {
                        let a2402 = v2;
                        let a2403 = 85u64;
                        if nat_lt(a2402, a2403) {
                            Ok(8u64)
                        } else {
                            let a2404 = v2;
                            let a2405 = 86u64;
                            if nat_lt(a2404, a2405) {
                                Ok(12u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    } else {
                        let a2406 = v2;
                        let a2407 = 89u64;
                        if nat_lt(a2406, a2407) {
                            let a2408 = v2;
                            let a2409 = 88u64;
                            if nat_lt(a2408, a2409) {
                                Ok(1u64)
                            } else {
                                Ok(13u64)
                            }
                        } else {
                            let a2410 = v2;
                            let a2411 = 90u64;
                            if nat_lt(a2410, a2411) {
                                Ok(3u64)
                            } else {
                                Ok(0u64)
                            }
                        }
                    }
                } else {
                    let a2412 = v2;
                    let a2413 = 94u64;
                    if nat_lt(a2412, a2413) {
                        let a2414 = v2;
                        let a2415 = 92u64;
                        if nat_lt(a2414, a2415) {
                            Ok(10u64)
                        } else {
                            let a2416 = v2;
                            let a2417 = 93u64;
                            if nat_lt(a2416, a2417) {
                                Ok(2u64)
                            } else {
                                Ok(6u64)
                            }
                        }
                    } else {
                        let a2418 = v2;
                        let a2419 = 96u64;
                        if nat_lt(a2418, a2419) {
                            let a2420 = v2;
                            let a2421 = 95u64;
                            if nat_lt(a2420, a2421) {
                                Ok(4u64)
                            } else {
                                Ok(7u64)
                            }
                        } else {
                            let a2422 = v2;
                            let a2423 = 97u64;
                            if nat_lt(a2422, a2423) {
                                Ok(11u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    }
                }
            } else {
                let a2424 = v2;
                let a2425 = 105u64;
                if nat_lt(a2424, a2425) {
                    let a2426 = v2;
                    let a2427 = 101u64;
                    if nat_lt(a2426, a2427) {
                        let a2428 = v2;
                        let a2429 = 99u64;
                        if nat_lt(a2428, a2429) {
                            Ok(5u64)
                        } else {
                            let a2430 = v2;
                            let a2431 = 100u64;
                            if nat_lt(a2430, a2431) {
                                Ok(0u64)
                            } else {
                                Ok(1u64)
                            }
                        }
                    } else {
                        let a2432 = v2;
                        let a2433 = 103u64;
                        if nat_lt(a2432, a2433) {
                            let a2434 = v2;
                            let a2435 = 102u64;
                            if nat_lt(a2434, a2435) {
                                Ok(9u64)
                            } else {
                                Ok(8u64)
                            }
                        } else {
                            let a2436 = v2;
                            let a2437 = 104u64;
                            if nat_lt(a2436, a2437) {
                                Ok(6u64)
                            } else {
                                Ok(14u64)
                            }
                        }
                    }
                } else {
                    let a2438 = v2;
                    let a2439 = 108u64;
                    if nat_lt(a2438, a2439) {
                        let a2440 = v2;
                        let a2441 = 106u64;
                        if nat_lt(a2440, a2441) {
                            Ok(10u64)
                        } else {
                            let a2442 = v2;
                            let a2443 = 107u64;
                            if nat_lt(a2442, a2443) {
                                Ok(2u64)
                            } else {
                                Ok(12u64)
                            }
                        }
                    } else {
                        let a2444 = v2;
                        let a2445 = 110u64;
                        if nat_lt(a2444, a2445) {
                            let a2446 = v2;
                            let a2447 = 109u64;
                            if nat_lt(a2446, a2447) {
                                Ok(3u64)
                            } else {
                                Ok(4u64)
                            }
                        } else {
                            let a2448 = v2;
                            let a2449 = 111u64;
                            if nat_lt(a2448, a2449) {
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

pub fn f81(v0: u64) -> u32 {
    let a2450 = v0;
    let a2451 = 12u64;
    if nat_lt(a2450, a2451) {
        let a2452 = v0;
        let a2453 = 9u64;
        if nat_lt(a2452, a2453) {
            let a2454 = v0;
            let a2455 = 8u64;
            if nat_lt(a2454, a2455) {
                7u32
            } else {
                8u32
            }
        } else {
            let a2456 = v0;
            let a2457 = 10u64;
            if nat_lt(a2456, a2457) {
                9u32
            } else {
                let a2458 = v0;
                let a2459 = 11u64;
                if nat_lt(a2458, a2459) {
                    10u32
                } else {
                    11u32
                }
            }
        }
    } else {
        let a2460 = v0;
        let a2461 = 14u64;
        if nat_lt(a2460, a2461) {
            let a2462 = v0;
            let a2463 = 13u64;
            if nat_lt(a2462, a2463) {
                12u32
            } else {
                13u32
            }
        } else {
            let a2464 = v0;
            let a2465 = 15u64;
            if nat_lt(a2464, a2465) {
                14u32
            } else {
                let a2466 = v0;
                let a2467 = 16u64;
                if nat_lt(a2466, a2467) {
                    15u32
                } else {
                    16u32
                }
            }
        }
    }
}

pub fn f82(v0: Adt1, v1: u64) -> u32 {
    let a2468 = v1;
    let a2469 = 4u64;
    if nat_lt(a2468, a2469) {
        let a2470 = v1;
        let a2471 = 2u64;
        if nat_lt(a2470, a2471) {
            let a2472 = v1;
            let a2473 = 1u64;
            if nat_lt(a2472, a2473) {
                match v0 {
                    Adt1::C0(h2474, _, _, _, _, _, _, _) => {
                        h2474
                    }
                }
            } else {
                match v0 {
                    Adt1::C0(_, h2475, _, _, _, _, _, _) => {
                        h2475
                    }
                }
            }
        } else {
            let a2476 = v1;
            let a2477 = 3u64;
            if nat_lt(a2476, a2477) {
                match v0 {
                    Adt1::C0(_, _, h2478, _, _, _, _, _) => {
                        h2478
                    }
                }
            } else {
                match v0 {
                    Adt1::C0(_, _, _, h2479, _, _, _, _) => {
                        h2479
                    }
                }
            }
        }
    } else {
        let a2480 = v1;
        let a2481 = 6u64;
        if nat_lt(a2480, a2481) {
            let a2482 = v1;
            let a2483 = 5u64;
            if nat_lt(a2482, a2483) {
                match v0 {
                    Adt1::C0(_, _, _, _, h2484, _, _, _) => {
                        h2484
                    }
                }
            } else {
                match v0 {
                    Adt1::C0(_, _, _, _, _, h2485, _, _) => {
                        h2485
                    }
                }
            }
        } else {
            let a2486 = v1;
            let a2487 = 7u64;
            if nat_lt(a2486, a2487) {
                match v0 {
                    Adt1::C0(_, _, _, _, _, _, h2488, _) => {
                        h2488
                    }
                }
            } else {
                match v0 {
                    Adt1::C0(_, _, _, _, _, _, _, h2489) => {
                        h2489
                    }
                }
            }
        }
    }
}

pub fn f83(v0: Adt11, v1: u32, v2: u32) -> u32 {
    let m2490 = v0;
    match m2490 {
        Adt11::C0 => {
            f84(v1, v2)
        }
        Adt11::C1 => {
            f84(v1, v2)
        }
        Adt11::C2 => {
            f85(v1, v2)
        }
        Adt11::C3 => {
            f86(v1, v2)
        }
        Adt11::C4 => {
            f85(v1, v2)
        }
    }
}

pub fn f84(v0: u32, v1: u32) -> u32 {
    let m2493 = {
        let a2491 = f87(v0);
        let a2492 = f87(v1);
        fixed_u64::checked_add(a2491, a2492)
    };
    match m2493 {
        None => {
            0u32
        }
        Some(v2) => {
            f88(v2)
        }
    }
}

pub fn f85(v0: u32, v1: u32) -> u32 {
    let a2494 = v0;
    let a2495 = v1;
    fixed_u32::bit_xor(a2494, a2495)
}

pub fn f86(v0: u32, v1: u32) -> u32 {
    let m2498 = {
        let a2496 = 32u32;
        let a2497 = v1;
        fixed_u32::checked_sub(a2496, a2497)
    };
    match m2498 {
        None => {
            v0
        }
        Some(v2) => {
            f89(v0, v1, v2)
        }
    }
}

pub fn f87(v0: u32) -> u64 {
    let m2500 = {
        let a2499 = v0;
        fixed_u64::convert(i128::from(a2499))
    };
    match m2500 {
        None => {
            0u64
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f88(v0: u64) -> u32 {
    let m2504 = {
        let a2503 = {
            let a2501 = v0;
            let a2502 = 4294967295u64;
            fixed_u64::bit_and(a2501, a2502)
        };
        fixed_u32::convert(i128::from(a2503))
    };
    match m2504 {
        None => {
            0u32
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f89(v0: u32, v1: u32, v2: u32) -> u32 {
    let a2505 = f27(v0, v1);
    let a2506 = f70(v0, v2);
    fixed_u32::bit_or(a2505, a2506)
}

pub fn run(p0: Bytes) -> R<Bytes> {
    f0(p0)
}
