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
    F7,
}

impl Fn0 {
    pub fn apply(&self, p0: List<Bytes>, p1: Adt2) -> List<Bytes> {
        match self {
            Fn0::F7 => f7(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn1 {
    F10,
    F55,
}

impl Fn1 {
    pub fn apply(&self, p0: List<Bytes>, p1: Bytes) -> List<Bytes> {
        match self {
            Fn1::F10 => f10(p0, p1),
            Fn1::F55 => f55(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn2 {
    F13(u32, Adt1),
}

impl Fn2 {
    pub fn apply(&self, p0: (Adt7, (List<Adt2>, u64)), p1: Adt5) -> R<(Adt7, (List<Adt2>, u64))> {
        match self {
            Fn2::F13(k0, k1) => f13(*k0, k1.clone(), p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn3 {
    F15((Adt7, (List<Adt2>, u64))),
}

impl Fn3 {
    pub fn apply(&self, p0: (u64, List<Adt2>)) -> R<(u64, List<Adt2>)> {
        match self {
            Fn3::F15(k0) => f15(k0.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn4 {
    F19(u32, Bytes, Adt1, Adt6),
}

impl Fn4 {
    pub fn apply(&self, p0: (u64, (List<Adt5>, List<Adt2>))) -> R<(u64, (List<Adt5>, List<Adt2>))> {
        match self {
            Fn4::F19(k0, k1, k2, k3) => f19(*k0, k1.clone(), k2.clone(), k3.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn5 {
    F22,
}

impl Fn5 {
    pub fn apply(&self, p0: List<Bytes>) -> Option<List<Bytes>> {
        match self {
            Fn5::F22 => f22(p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn6 {
    F25(u32, u64, u64, Adt1, u64, Bytes),
}

impl Fn6 {
    pub fn apply(&self, p0: (u64, (Adt1, List<Adt2>))) -> R<(u64, (Adt1, List<Adt2>))> {
        match self {
            Fn6::F25(k0, k1, k2, k3, k4, k5) => f25(*k0, *k1, *k2, k3.clone(), *k4, k5.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn7 {
    F37,
}

impl Fn7 {
    pub fn apply(&self, p0: List<Adt2>, p1: Adt2) -> List<Adt2> {
        match self {
            Fn7::F37 => f37(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn8 {
    F50(u32, Adt1),
}

impl Fn8 {
    pub fn apply(&self, p0: Adt8) -> R<Option<Adt8>> {
        match self {
            Fn8::F50(k0, k1) => f50(*k0, k1.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn9 {
    F54,
}

impl Fn9 {
    pub fn apply(&self, p0: (Option<Bytes>, List<Bytes>), p1: Bytes) -> (Option<Bytes>, List<Bytes>) {
        match self {
            Fn9::F54 => f54(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn10 {
    F58,
}

impl Fn10 {
    pub fn apply(&self, p0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
        match self {
            Fn10::F58 => f58(p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn11 {
    F66,
}

impl Fn11 {
    pub fn apply(&self, p0: Adt4, p1: Adt10) -> Adt4 {
        match self {
            Fn11::F66 => f66(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn12 {
    F68(Adt1, Adt4),
}

impl Fn12 {
    pub fn apply(&self, p0: (u64, Adt12)) -> R<(u64, Adt12)> {
        match self {
            Fn12::F68(k0, k1) => f68(k0.clone(), k1.clone(), p0),
        }
    }
}

pub fn f0(v0: Bytes, v1: u64) -> R<Bytes> {
    Ok(f1(f2(f3(f4(f5(), 0u32), v0)?, v1)?))
}

pub fn f1(v0: Adt0) -> Bytes {
    let v1: List<Bytes> = f6(Fn0::F7, List::<Bytes>::nil(), match v0.clone() {
        Adt0::C0(_, _, _, _, _, _, h1) => {
            h1
        }
    });
    let v2: Bytes = f8(f9(Fn1::F10, List::<Bytes>::nil(), v1));
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
    let v5: (Adt7, List<Adt2>) = f11(v2.clone(), v3, v4.clone(), 0u64, {
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
    let v6: (Adt7, (List<Adt2>, u64)) = f12(Fn2::F13(v3, v2.clone()), ({
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
    let v7: (u64, List<Adt2>) = f14(Fn3::F15(v6.clone()), {
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
    Ok(Adt0::C0(f16(v3), {
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
    }, f17({
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
    let v7: (u64, (List<Adt5>, List<Adt2>)) = f18(Fn4::F19(v5, v2.clone(), v6.clone(), v0.clone()), v4, (0u64, (match v0.clone() {
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

pub fn f4(v0: Adt1, v1: u32) -> Adt6 {
    Adt6::C0(v0, v1, 0u64, Bytes::lit(&[]), List::<Adt5>::nil(), List::<Adt2>::nil(), List::<Adt2>::nil())
}

pub fn f5() -> Adt1 {
    Adt1::C0(1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, 1359893119u32, 2600822924u32, 528734635u32, 1541459225u32)
}

pub fn f6(v0: Fn0, p67: List<Bytes>, p68: List<Adt2>) -> List<Bytes> {
    let m66 = (p67, p68);
    {
        let mut m66 = m66;
        loop {
            let step = {
                let (v1, v2) = m66;
                let m64 = v2;
                match m64.uncons() {
                    None => {
                        Err::<(List<Bytes>, List<Adt2>), List<Bytes>>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(List<Bytes>, List<Adt2>), List<Bytes>>(({
                            let c65: Fn0 = v0.clone();
                            c65.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m66 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f7(v0: List<Bytes>, v1: Adt2) -> List<Bytes> {
    let m69 = v1;
    match m69 {
        Adt2::C0(v2) => {
            let a71 = match v2.clone() {
                Adt3::C0(h70, _, _, _, _, _, _, _, _, _, _, _) => {
                    h70
                }
            };
            let a72 = 2u64;
            if nat_eq(a71, a72) {
                List::cons(f20(match v2 {
                    Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h73) => {
                        h73
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

pub fn f8(v0: List<Bytes>) -> Bytes {
    let m75 = {
        let (h74, _) = f21(Fn5::F22, 64u64, v0);
        h74
    };
    match m75.uncons() {
        None => {
            Bytes::lit(&[])
        }
        Some((v1, _)) => {
            v1
        }
    }
}

pub fn f9(v0: Fn1, p79: List<Bytes>, p80: List<Bytes>) -> List<Bytes> {
    let m78 = (p79, p80);
    {
        let mut m78 = m78;
        loop {
            let step = {
                let (v1, v2) = m78;
                let m76 = v2;
                match m76.uncons() {
                    None => {
                        Err::<(List<Bytes>, List<Bytes>), List<Bytes>>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(List<Bytes>, List<Bytes>), List<Bytes>>(({
                            let c77: Fn1 = v0.clone();
                            c77.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m78 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f10(v0: List<Bytes>, v1: Bytes) -> List<Bytes> {
    List::cons(v1, v0)
}

pub fn f11(v0: Adt1, v1: u32, v2: Bytes, v3: u64, v4: u64, v5: u64, v6: List<Adt2>) -> R<(Adt7, List<Adt2>)> {
    let v7: u64 = {
        let a81 = v3;
        let a82 = v4;
        nat_add(a81, a82)?
    };
    let v8: u64 = f23(v4)?;
    let v9: (u64, (Adt1, List<Adt2>)) = f24(Fn6::F25(v1, v3, v5, v0.clone(), v7, v2.clone()), {
        let a83 = v8;
        let a84 = 64u64;
        let a85 = 0u64;
        nat_quot(a83, a84, a85)
    }, (0u64, (v0.clone(), v6)))?;
    Ok((f26(v0, v1, v2, v7, v3, v5, v8, {
        let (h86, _) = {
            let (_, h87) = v9.clone();
            h87
        };
        h86
    }, {
        let a88 = v4;
        let a89 = v8;
        nat_sub(a88, a89)
    }, {
        let a90 = f27();
        let a91 = f28(v8);
        fixed_u32::bit_or(a90, a91)
    })?, {
        let (_, h92) = {
            let (_, h93) = v9;
            h93
        };
        h92
    }))
}

pub fn f12(v0: Fn2, p97: (Adt7, (List<Adt2>, u64)), p98: List<Adt5>) -> R<(Adt7, (List<Adt2>, u64))> {
    let m96 = (p97, p98);
    {
        let mut m96 = m96;
        loop {
            let step = {
                let (v1, v2) = m96;
                let m94 = v2;
                match m94.uncons() {
                    None => {
                        Err::<((Adt7, (List<Adt2>, u64)), List<Adt5>), R<(Adt7, (List<Adt2>, u64))>>(Ok(v1))
                    }
                    Some((v3, v4)) => {
                        Ok::<((Adt7, (List<Adt2>, u64)), List<Adt5>), R<(Adt7, (List<Adt2>, u64))>>(({
                            let c95: Fn2 = v0.clone();
                            c95.apply(v1, v3)?
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

pub fn f13(v0: u32, v1: Adt1, v2: (Adt7, (List<Adt2>, u64)), v3: Adt5) -> R<(Adt7, (List<Adt2>, u64))> {
    let v4: Adt3 = f29({
        let (h99, _) = v2.clone();
        h99
    })?;
    Ok((f30(v3.clone(), Adt5::C0(f31(match v4.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h100) => {
            h100
        }
    }), {
        let m103 = {
            let (h102, _) = v2.clone();
            h102
        };
        match m103 {
            Adt7::C0(_, h101, _, _, _, _, _, _, _, _) => {
                h101
            }
        }
    }, {
        let m106 = {
            let (h105, _) = v2.clone();
            h105
        };
        match m106 {
            Adt7::C0(_, _, h104, _, _, _, _, _, _, _) => {
                h104
            }
        }
    }), v1, v0)?, (List::cons(Adt2::C2(v3, {
        let (_, h107) = {
            let (_, h108) = v2.clone();
            h108
        };
        h107
    }), List::cons(Adt2::C0(v4), {
        let (h109, _) = {
            let (_, h110) = v2.clone();
            h110
        };
        h109
    })), {
        let a113 = {
            let (_, h111) = {
                let (_, h112) = v2;
                h112
            };
            h111
        };
        let a114 = 1u64;
        nat_sub(a113, a114)
    })))
}

pub fn f14(v0: Fn3, p118: u64, p119: (u64, List<Adt2>)) -> R<(u64, List<Adt2>)> {
    let m117 = (p118, p119);
    {
        let mut m117 = m117;
        loop {
            let step = {
                let (v1, v2) = m117;
                let m115 = v1;
                if m115 == 0 {
                    Err::<(u64, (u64, List<Adt2>)), R<(u64, List<Adt2>)>>(Ok(v2))
                } else {
                    let v3 = m115 - 1;
                    Ok::<(u64, (u64, List<Adt2>)), R<(u64, List<Adt2>)>>((v3, {
                        let c116: Fn3 = v0.clone();
                        c116.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m117 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f15(v0: (Adt7, (List<Adt2>, u64)), v1: (u64, List<Adt2>)) -> R<(u64, List<Adt2>)> {
    Ok(({
        let a121 = {
            let (h120, _) = v1.clone();
            h120
        };
        let a122 = 1u64;
        nat_add(a121, a122)?
    }, List::cons(Adt2::C0(f29(f32({
        let (h123, _) = v0;
        h123
    }, {
        let (h124, _) = v1;
        h124
    }))?), {
        let (_, h125) = v1;
        h125
    })))
}

pub fn f16(v0: u32) -> u64 {
    let a126 = v0;
    let a127 = f33();
    if equal(a126, a127) {
        1u64
    } else {
        let a128 = v0;
        let a129 = f34();
        if equal(a128, a129) {
            2u64
        } else {
            let a130 = v0;
            let a131 = f35();
            if equal(a130, a131) {
                3u64
            } else {
                0u64
            }
        }
    }
}

pub fn f17(v0: List<Adt2>) -> List<Adt2> {
    f36(Fn7::F37, List::<Adt2>::nil(), v0)
}

pub fn f18(v0: Fn4, p135: u64, p136: (u64, (List<Adt5>, List<Adt2>))) -> R<(u64, (List<Adt5>, List<Adt2>))> {
    let m134 = (p135, p136);
    {
        let mut m134 = m134;
        loop {
            let step = {
                let (v1, v2) = m134;
                let m132 = v1;
                if m132 == 0 {
                    Err::<(u64, (u64, (List<Adt5>, List<Adt2>))), R<(u64, (List<Adt5>, List<Adt2>))>>(Ok(v2))
                } else {
                    let v3 = m132 - 1;
                    Ok::<(u64, (u64, (List<Adt5>, List<Adt2>))), R<(u64, (List<Adt5>, List<Adt2>))>>((v3, {
                        let c133: Fn4 = v0.clone();
                        c133.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m134 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f19(v0: u32, v1: Bytes, v2: Adt1, v3: Adt6, v4: (u64, (List<Adt5>, List<Adt2>))) -> R<(u64, (List<Adt5>, List<Adt2>))> {
    let v5: u64 = {
        let a139 = match v3 {
            Adt6::C0(_, _, h137, _, _, _, _) => {
                h137
            }
        };
        let a140 = {
            let (h138, _) = v4.clone();
            h138
        };
        nat_add(a139, a140)?
    };
    let v6: (Adt7, List<Adt2>) = f11(v2.clone(), v0, v1, {
        let a142 = {
            let (h141, _) = v4.clone();
            h141
        };
        let a143 = 1024u64;
        nat_mul(a142, a143)?
    }, 1024u64, v5, {
        let (_, h144) = {
            let (_, h145) = v4.clone();
            h145
        };
        h144
    })?;
    let v7: Adt3 = f29({
        let (h146, _) = v6;
        h146
    })?;
    Ok(({
        let a148 = {
            let (h147, _) = v4;
            h147
        };
        let a149 = 1u64;
        nat_add(a148, a149)?
    }, f38(v2, v0, Adt8::C0(Adt5::C0(f31(match v7.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h150) => {
            h150
        }
    }), v5, 1u64), {
        let a151 = v5;
        let a152 = 1u64;
        nat_add(a151, a152)?
    }, {
        let (h153, _) = {
            let (_, h154) = v4;
            h154
        };
        h153
    }, List::cons(Adt2::C0(v7), {
        let (_, h155) = v6;
        h155
    })))?))
}

pub fn f20(v0: Adt4) -> Bytes {
    let a200 = f39(match v0.clone() {
        Adt4::C0(h156, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h156
        }
    });
    let a201 = {
        let a198 = f39(match v0.clone() {
            Adt4::C0(_, h157, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                h157
            }
        });
        let a199 = {
            let a196 = f39(match v0.clone() {
                Adt4::C0(_, _, h158, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                    h158
                }
            });
            let a197 = {
                let a194 = f39(match v0.clone() {
                    Adt4::C0(_, _, _, h159, _, _, _, _, _, _, _, _, _, _, _, _) => {
                        h159
                    }
                });
                let a195 = {
                    let a192 = f39(match v0.clone() {
                        Adt4::C0(_, _, _, _, h160, _, _, _, _, _, _, _, _, _, _, _) => {
                            h160
                        }
                    });
                    let a193 = {
                        let a190 = f39(match v0.clone() {
                            Adt4::C0(_, _, _, _, _, h161, _, _, _, _, _, _, _, _, _, _) => {
                                h161
                            }
                        });
                        let a191 = {
                            let a188 = f39(match v0.clone() {
                                Adt4::C0(_, _, _, _, _, _, h162, _, _, _, _, _, _, _, _, _) => {
                                    h162
                                }
                            });
                            let a189 = {
                                let a186 = f39(match v0.clone() {
                                    Adt4::C0(_, _, _, _, _, _, _, h163, _, _, _, _, _, _, _, _) => {
                                        h163
                                    }
                                });
                                let a187 = {
                                    let a184 = f39(match v0.clone() {
                                        Adt4::C0(_, _, _, _, _, _, _, _, h164, _, _, _, _, _, _, _) => {
                                            h164
                                        }
                                    });
                                    let a185 = {
                                        let a182 = f39(match v0.clone() {
                                            Adt4::C0(_, _, _, _, _, _, _, _, _, h165, _, _, _, _, _, _) => {
                                                h165
                                            }
                                        });
                                        let a183 = {
                                            let a180 = f39(match v0.clone() {
                                                Adt4::C0(_, _, _, _, _, _, _, _, _, _, h166, _, _, _, _, _) => {
                                                    h166
                                                }
                                            });
                                            let a181 = {
                                                let a178 = f39(match v0.clone() {
                                                    Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h167, _, _, _, _) => {
                                                        h167
                                                    }
                                                });
                                                let a179 = {
                                                    let a176 = f39(match v0.clone() {
                                                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h168, _, _, _) => {
                                                            h168
                                                        }
                                                    });
                                                    let a177 = {
                                                        let a174 = f39(match v0.clone() {
                                                            Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h169, _, _) => {
                                                                h169
                                                            }
                                                        });
                                                        let a175 = {
                                                            let a172 = f39(match v0.clone() {
                                                                Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h170, _) => {
                                                                    h170
                                                                }
                                                            });
                                                            let a173 = f39(match v0 {
                                                                Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h171) => {
                                                                    h171
                                                                }
                                                            });
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
}

pub fn f21(v0: Fn5, p206: u64, p207: List<Bytes>) -> (List<Bytes>, bool) {
    let m205 = (p206, p207);
    {
        let mut m205 = m205;
        loop {
            let step = {
                let (v1, v2) = m205;
                let m202 = v1;
                if m202 == 0 {
                    Err::<(u64, List<Bytes>), (List<Bytes>, bool)>((v2, false))
                } else {
                    let v3 = m202 - 1;
                    let m204 = {
                        let c203: Fn5 = v0.clone();
                        c203.apply(v2.clone())
                    };
                    match m204 {
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
                Ok(next) => { m205 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f22(v0: List<Bytes>) -> Option<List<Bytes>> {
    let a209 = {
        let a208 = v0.clone();
        length_list(a208)
    };
    let a210 = 1u64;
    if nat_le(a209, a210) {
        None::<List<Bytes>>
    } else {
        Some(f40(v0))
    }
}

pub fn f23(v0: u64) -> R<u64> {
    let a211 = v0;
    let a212 = 0u64;
    if nat_eq(a211, a212) {
        Ok(0u64)
    } else {
        let a218 = {
            let a215 = {
                let a213 = v0;
                let a214 = 1u64;
                nat_sub(a213, a214)
            };
            let a216 = 64u64;
            let a217 = 0u64;
            nat_quot(a215, a216, a217)
        };
        let a219 = 64u64;
        nat_mul(a218, a219)
    }
}

pub fn f24(v0: Fn6, p223: u64, p224: (u64, (Adt1, List<Adt2>))) -> R<(u64, (Adt1, List<Adt2>))> {
    let m222 = (p223, p224);
    {
        let mut m222 = m222;
        loop {
            let step = {
                let (v1, v2) = m222;
                let m220 = v1;
                if m220 == 0 {
                    Err::<(u64, (u64, (Adt1, List<Adt2>))), R<(u64, (Adt1, List<Adt2>))>>(Ok(v2))
                } else {
                    let v3 = m220 - 1;
                    Ok::<(u64, (u64, (Adt1, List<Adt2>))), R<(u64, (Adt1, List<Adt2>))>>((v3, {
                        let c221: Fn6 = v0.clone();
                        c221.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m222 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f25(v0: u32, v1: u64, v2: u64, v3: Adt1, v4: u64, v5: Bytes, v6: (u64, (Adt1, List<Adt2>))) -> R<(u64, (Adt1, List<Adt2>))> {
    let v7: u64 = {
        let (h225, _) = v6.clone();
        h225
    };
    let v8: Adt3 = f29(f26(v3, v0, v5, v4, v1, v2, v7, {
        let (h226, _) = {
            let (_, h227) = v6.clone();
            h227
        };
        h226
    }, 64u64, f28(v7))?)?;
    Ok(({
        let a228 = v7;
        let a229 = 64u64;
        nat_add(a228, a229)?
    }, (f31(match v8.clone() {
        Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h230) => {
            h230
        }
    }), List::cons(Adt2::C0(v8), {
        let (_, h231) = {
            let (_, h232) = v6;
            h232
        };
        h231
    }))))
}

pub fn f26(_: Adt1, v1: u32, v2: Bytes, v3: u64, v4: u64, v5: u64, v6: u64, v7: Adt1, v8: u64, v9: u32) -> R<Adt7> {
    Ok(Adt7::C0(0u64, v5, 1u64, {
        let a233 = v6;
        let a234 = 64u64;
        let a235 = 0u64;
        nat_quot(a233, a234, a235)
    }, {
        let a238 = {
            let a236 = v5;
            let a237 = 1024u64;
            nat_mul(a236, a237)?
        };
        let a239 = v6;
        nat_add(a238, a239)?
    }, v7, f41(v2, v3, {
        let a240 = v4;
        let a241 = v6;
        nat_add(a240, a241)?
    })?, v5, f42(v8, 7u64), {
        let a242 = v1;
        let a243 = v9;
        fixed_u32::bit_or(a242, a243)
    }))
}

pub fn f27() -> u32 {
    2u32
}

pub fn f28(v0: u64) -> u32 {
    let a244 = v0;
    let a245 = 0u64;
    if nat_eq(a244, a245) {
        f43()
    } else {
        0u32
    }
}

pub fn f29(v0: Adt7) -> R<Adt3> {
    let v1: u32 = f44(match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, h246, _, _) => {
            h246
        }
    });
    let v2: u32 = f45(match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, h247, _, _) => {
            h247
        }
    });
    Ok(Adt3::C0(match v0.clone() {
        Adt7::C0(h248, _, _, _, _, _, _, _, _, _) => {
            h248
        }
    }, match v0.clone() {
        Adt7::C0(_, h249, _, _, _, _, _, _, _, _) => {
            h249
        }
    }, match v0.clone() {
        Adt7::C0(_, _, h250, _, _, _, _, _, _, _) => {
            h250
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, h251, _, _, _, _, _, _) => {
            h251
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, h252, _, _, _, _, _) => {
            h252
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, h253, _, _, _, _) => {
            h253
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, h254, _, _, _) => {
            h254
        }
    }, v1, v2, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, _, h255, _) => {
            h255
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, _, _, h256) => {
            h256
        }
    }, f46(match v0.clone() {
        Adt7::C0(_, _, _, _, _, h257, _, _, _, _) => {
            h257
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, h258, _, _, _) => {
            h258
        }
    }, v1, v2, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, _, h259, _) => {
            h259
        }
    }, match v0 {
        Adt7::C0(_, _, _, _, _, _, _, _, _, h260) => {
            h260
        }
    })?))
}

pub fn f30(v0: Adt5, v1: Adt5, v2: Adt1, v3: u32) -> R<Adt7> {
    Ok(Adt7::C0(1u64, match v0.clone() {
        Adt5::C0(_, h261, _) => {
            h261
        }
    }, {
        let a264 = match v0.clone() {
            Adt5::C0(_, _, h262) => {
                h262
            }
        };
        let a265 = match v1.clone() {
            Adt5::C0(_, _, h263) => {
                h263
            }
        };
        nat_add(a264, a265)?
    }, 0u64, 0u64, v2, Adt4::C0({
        let m268 = match v0.clone() {
            Adt5::C0(h267, _, _) => {
                h267
            }
        };
        match m268 {
            Adt1::C0(h266, _, _, _, _, _, _, _) => {
                h266
            }
        }
    }, {
        let m271 = match v0.clone() {
            Adt5::C0(h270, _, _) => {
                h270
            }
        };
        match m271 {
            Adt1::C0(_, h269, _, _, _, _, _, _) => {
                h269
            }
        }
    }, {
        let m274 = match v0.clone() {
            Adt5::C0(h273, _, _) => {
                h273
            }
        };
        match m274 {
            Adt1::C0(_, _, h272, _, _, _, _, _) => {
                h272
            }
        }
    }, {
        let m277 = match v0.clone() {
            Adt5::C0(h276, _, _) => {
                h276
            }
        };
        match m277 {
            Adt1::C0(_, _, _, h275, _, _, _, _) => {
                h275
            }
        }
    }, {
        let m280 = match v0.clone() {
            Adt5::C0(h279, _, _) => {
                h279
            }
        };
        match m280 {
            Adt1::C0(_, _, _, _, h278, _, _, _) => {
                h278
            }
        }
    }, {
        let m283 = match v0.clone() {
            Adt5::C0(h282, _, _) => {
                h282
            }
        };
        match m283 {
            Adt1::C0(_, _, _, _, _, h281, _, _) => {
                h281
            }
        }
    }, {
        let m286 = match v0.clone() {
            Adt5::C0(h285, _, _) => {
                h285
            }
        };
        match m286 {
            Adt1::C0(_, _, _, _, _, _, h284, _) => {
                h284
            }
        }
    }, {
        let m289 = match v0 {
            Adt5::C0(h288, _, _) => {
                h288
            }
        };
        match m289 {
            Adt1::C0(_, _, _, _, _, _, _, h287) => {
                h287
            }
        }
    }, {
        let m292 = match v1.clone() {
            Adt5::C0(h291, _, _) => {
                h291
            }
        };
        match m292 {
            Adt1::C0(h290, _, _, _, _, _, _, _) => {
                h290
            }
        }
    }, {
        let m295 = match v1.clone() {
            Adt5::C0(h294, _, _) => {
                h294
            }
        };
        match m295 {
            Adt1::C0(_, h293, _, _, _, _, _, _) => {
                h293
            }
        }
    }, {
        let m298 = match v1.clone() {
            Adt5::C0(h297, _, _) => {
                h297
            }
        };
        match m298 {
            Adt1::C0(_, _, h296, _, _, _, _, _) => {
                h296
            }
        }
    }, {
        let m301 = match v1.clone() {
            Adt5::C0(h300, _, _) => {
                h300
            }
        };
        match m301 {
            Adt1::C0(_, _, _, h299, _, _, _, _) => {
                h299
            }
        }
    }, {
        let m304 = match v1.clone() {
            Adt5::C0(h303, _, _) => {
                h303
            }
        };
        match m304 {
            Adt1::C0(_, _, _, _, h302, _, _, _) => {
                h302
            }
        }
    }, {
        let m307 = match v1.clone() {
            Adt5::C0(h306, _, _) => {
                h306
            }
        };
        match m307 {
            Adt1::C0(_, _, _, _, _, h305, _, _) => {
                h305
            }
        }
    }, {
        let m310 = match v1.clone() {
            Adt5::C0(h309, _, _) => {
                h309
            }
        };
        match m310 {
            Adt1::C0(_, _, _, _, _, _, h308, _) => {
                h308
            }
        }
    }, {
        let m313 = match v1 {
            Adt5::C0(h312, _, _) => {
                h312
            }
        };
        match m313 {
            Adt1::C0(_, _, _, _, _, _, _, h311) => {
                h311
            }
        }
    }), 0u64, 64u32, {
        let a314 = v3;
        let a315 = f47();
        fixed_u32::bit_or(a314, a315)
    }))
}

pub fn f31(v0: Adt4) -> Adt1 {
    Adt1::C0(match v0.clone() {
        Adt4::C0(h316, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h316
        }
    }, match v0.clone() {
        Adt4::C0(_, h317, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h317
        }
    }, match v0.clone() {
        Adt4::C0(_, _, h318, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h318
        }
    }, match v0.clone() {
        Adt4::C0(_, _, _, h319, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h319
        }
    }, match v0.clone() {
        Adt4::C0(_, _, _, _, h320, _, _, _, _, _, _, _, _, _, _, _) => {
            h320
        }
    }, match v0.clone() {
        Adt4::C0(_, _, _, _, _, h321, _, _, _, _, _, _, _, _, _, _) => {
            h321
        }
    }, match v0.clone() {
        Adt4::C0(_, _, _, _, _, _, h322, _, _, _, _, _, _, _, _, _) => {
            h322
        }
    }, match v0 {
        Adt4::C0(_, _, _, _, _, _, _, h323, _, _, _, _, _, _, _, _) => {
            h323
        }
    })
}

pub fn f32(v0: Adt7, v1: u64) -> Adt7 {
    Adt7::C0(2u64, match v0.clone() {
        Adt7::C0(_, h324, _, _, _, _, _, _, _, _) => {
            h324
        }
    }, match v0.clone() {
        Adt7::C0(_, _, h325, _, _, _, _, _, _, _) => {
            h325
        }
    }, v1, match v0.clone() {
        Adt7::C0(_, _, _, _, h326, _, _, _, _, _) => {
            h326
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, h327, _, _, _, _) => {
            h327
        }
    }, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, h328, _, _, _) => {
            h328
        }
    }, v1, match v0.clone() {
        Adt7::C0(_, _, _, _, _, _, _, _, h329, _) => {
            h329
        }
    }, {
        let a331 = match v0 {
            Adt7::C0(_, _, _, _, _, _, _, _, _, h330) => {
                h330
            }
        };
        let a332 = f48();
        fixed_u32::bit_or(a331, a332)
    })
}

pub fn f33() -> u32 {
    16u32
}

pub fn f34() -> u32 {
    64u32
}

pub fn f35() -> u32 {
    32u32
}

pub fn f36(v0: Fn7, p336: List<Adt2>, p337: List<Adt2>) -> List<Adt2> {
    let m335 = (p336, p337);
    {
        let mut m335 = m335;
        loop {
            let step = {
                let (v1, v2) = m335;
                let m333 = v2;
                match m333.uncons() {
                    None => {
                        Err::<(List<Adt2>, List<Adt2>), List<Adt2>>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(List<Adt2>, List<Adt2>), List<Adt2>>(({
                            let c334: Fn7 = v0.clone();
                            c334.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m335 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f37(v0: List<Adt2>, v1: Adt2) -> List<Adt2> {
    List::cons(v1, v0)
}

pub fn f38(v0: Adt1, v1: u32, v2: Adt8) -> R<(List<Adt5>, List<Adt2>)> {
    let v3: Adt8 = {
        let (h338, _) = f49(Fn8::F50(v1, v0), 64u64, v2)?;
        h338
    };
    let v4: List<Adt5> = List::cons(match v3.clone() {
        Adt8::C0(h339, _, _, _) => {
            h339
        }
    }, match v3.clone() {
        Adt8::C0(_, _, h340, _) => {
            h340
        }
    });
    Ok((v4.clone(), List::cons(Adt2::C1(match v3.clone() {
        Adt8::C0(h341, _, _, _) => {
            h341
        }
    }, {
        let a342 = v4;
        length_list(a342)
    }), match v3 {
        Adt8::C0(_, _, _, h343) => {
            h343
        }
    })))
}

pub fn f39(v0: u32) -> Bytes {
    let a354 = f51({
        let a344 = v0;
        let a345 = 255u32;
        fixed_u32::bit_and(a344, a345)
    });
    let a355 = {
        let a352 = f51({
            let a346 = f52(v0, 8u32);
            let a347 = 255u32;
            fixed_u32::bit_and(a346, a347)
        });
        let a353 = {
            let a350 = f51({
                let a348 = f52(v0, 16u32);
                let a349 = 255u32;
                fixed_u32::bit_and(a348, a349)
            });
            let a351 = f51(f52(v0, 24u32));
            append_bytes(a350, a351)
        };
        append_bytes(a352, a353)
    };
    append_bytes(a354, a355)
}

pub fn f40(v0: List<Bytes>) -> List<Bytes> {
    let v1: (Option<Bytes>, List<Bytes>) = f53(Fn9::F54, (None::<Bytes>, List::<Bytes>::nil()), v0);
    let v3: List<Bytes> = {
        let m357 = {
            let (h356, _) = v1;
            h356
        };
        match m357 {
            None => {
                let (_, h359) = v1;
                h359
            }
            Some(v2) => {
                List::cons(v2, {
                    let (_, h358) = v1;
                    h358
                })
            }
        }
    };
    f9(Fn1::F55, List::<Bytes>::nil(), v3)
}

pub fn f41(v0: Bytes, v1: u64, v2: u64) -> R<Adt4> {
    Ok(Adt4::C0(f56(v0.clone(), v1, {
        let a360 = v2;
        let a361 = 0u64;
        nat_add(a360, a361)?
    })?, f56(v0.clone(), v1, {
        let a362 = v2;
        let a363 = 4u64;
        nat_add(a362, a363)?
    })?, f56(v0.clone(), v1, {
        let a364 = v2;
        let a365 = 8u64;
        nat_add(a364, a365)?
    })?, f56(v0.clone(), v1, {
        let a366 = v2;
        let a367 = 12u64;
        nat_add(a366, a367)?
    })?, f56(v0.clone(), v1, {
        let a368 = v2;
        let a369 = 16u64;
        nat_add(a368, a369)?
    })?, f56(v0.clone(), v1, {
        let a370 = v2;
        let a371 = 20u64;
        nat_add(a370, a371)?
    })?, f56(v0.clone(), v1, {
        let a372 = v2;
        let a373 = 24u64;
        nat_add(a372, a373)?
    })?, f56(v0.clone(), v1, {
        let a374 = v2;
        let a375 = 28u64;
        nat_add(a374, a375)?
    })?, f56(v0.clone(), v1, {
        let a376 = v2;
        let a377 = 32u64;
        nat_add(a376, a377)?
    })?, f56(v0.clone(), v1, {
        let a378 = v2;
        let a379 = 36u64;
        nat_add(a378, a379)?
    })?, f56(v0.clone(), v1, {
        let a380 = v2;
        let a381 = 40u64;
        nat_add(a380, a381)?
    })?, f56(v0.clone(), v1, {
        let a382 = v2;
        let a383 = 44u64;
        nat_add(a382, a383)?
    })?, f56(v0.clone(), v1, {
        let a384 = v2;
        let a385 = 48u64;
        nat_add(a384, a385)?
    })?, f56(v0.clone(), v1, {
        let a386 = v2;
        let a387 = 52u64;
        nat_add(a386, a387)?
    })?, f56(v0.clone(), v1, {
        let a388 = v2;
        let a389 = 56u64;
        nat_add(a388, a389)?
    })?, f56(v0, v1, {
        let a390 = v2;
        let a391 = 60u64;
        nat_add(a390, a391)?
    })?))
}

pub fn f42(v0: u64, v1: u64) -> u32 {
    let (_, h392) = {
        let (_, h393) = f57(Fn10::F58, v1, (v0, (1u32, 0u32)));
        h393
    };
    h392
}

pub fn f43() -> u32 {
    1u32
}

pub fn f44(v0: u64) -> u32 {
    f42({
        let a394 = v0;
        let a395 = 4294967296u64;
        let a396 = 0u64;
        nat_rem(a394, a395, a396)
    }, 32u64)
}

pub fn f45(v0: u64) -> u32 {
    let a397 = v0;
    let a398 = 4294967296u64;
    if nat_lt(a397, a398) {
        0u32
    } else {
        f42({
            let a399 = v0;
            let a400 = 4294967296u64;
            let a401 = 0u64;
            nat_quot(a399, a400, a401)
        }, 32u64)
    }
}

pub fn f46(v0: Adt1, v1: Adt4, v2: u32, v3: u32, v4: u32, v5: u32) -> R<Adt4> {
    Ok(f59(f60(v0, v1, v2, v3, v4, v5)?))
}

pub fn f47() -> u32 {
    4u32
}

pub fn f48() -> u32 {
    8u32
}

pub fn f49(v0: Fn8, p406: u64, p407: Adt8) -> R<(Adt8, bool)> {
    let m405 = (p406, p407);
    {
        let mut m405 = m405;
        loop {
            let step = {
                let (v1, v2) = m405;
                let m402 = v1;
                if m402 == 0 {
                    Err::<(u64, Adt8), R<(Adt8, bool)>>(Ok((v2, false)))
                } else {
                    let v3 = m402 - 1;
                    let m404 = {
                        let c403: Fn8 = v0.clone();
                        c403.apply(v2.clone())?
                    };
                    match m404 {
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
                Ok(next) => { m405 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f50(v0: u32, v1: Adt1, v2: Adt8) -> R<Option<Adt8>> {
    f61(v1, v0, v2)
}

pub fn f51(v0: u32) -> Bytes {
    let a410 = {
        let a408 = v0;
        let a409 = 128u32;
        fixed_u32::bit_and(a408, a409)
    };
    let a411 = 0u32;
    if equal(a410, a411) {
        let a414 = {
            let a412 = v0;
            let a413 = 64u32;
            fixed_u32::bit_and(a412, a413)
        };
        let a415 = 0u32;
        if equal(a414, a415) {
            let a418 = {
                let a416 = v0;
                let a417 = 32u32;
                fixed_u32::bit_and(a416, a417)
            };
            let a419 = 0u32;
            if equal(a418, a419) {
                let a422 = {
                    let a420 = v0;
                    let a421 = 16u32;
                    fixed_u32::bit_and(a420, a421)
                };
                let a423 = 0u32;
                if equal(a422, a423) {
                    let a426 = {
                        let a424 = v0;
                        let a425 = 8u32;
                        fixed_u32::bit_and(a424, a425)
                    };
                    let a427 = 0u32;
                    if equal(a426, a427) {
                        let a430 = {
                            let a428 = v0;
                            let a429 = 4u32;
                            fixed_u32::bit_and(a428, a429)
                        };
                        let a431 = 0u32;
                        if equal(a430, a431) {
                            let a434 = {
                                let a432 = v0;
                                let a433 = 2u32;
                                fixed_u32::bit_and(a432, a433)
                            };
                            let a435 = 0u32;
                            if equal(a434, a435) {
                                let a438 = {
                                    let a436 = v0;
                                    let a437 = 1u32;
                                    fixed_u32::bit_and(a436, a437)
                                };
                                let a439 = 0u32;
                                if equal(a438, a439) {
                                    Bytes::lit(&[0x00])
                                } else {
                                    Bytes::lit(&[0x01])
                                }
                            } else {
                                let a442 = {
                                    let a440 = v0;
                                    let a441 = 1u32;
                                    fixed_u32::bit_and(a440, a441)
                                };
                                let a443 = 0u32;
                                if equal(a442, a443) {
                                    Bytes::lit(&[0x02])
                                } else {
                                    Bytes::lit(&[0x03])
                                }
                            }
                        } else {
                            let a446 = {
                                let a444 = v0;
                                let a445 = 2u32;
                                fixed_u32::bit_and(a444, a445)
                            };
                            let a447 = 0u32;
                            if equal(a446, a447) {
                                let a450 = {
                                    let a448 = v0;
                                    let a449 = 1u32;
                                    fixed_u32::bit_and(a448, a449)
                                };
                                let a451 = 0u32;
                                if equal(a450, a451) {
                                    Bytes::lit(&[0x04])
                                } else {
                                    Bytes::lit(&[0x05])
                                }
                            } else {
                                let a454 = {
                                    let a452 = v0;
                                    let a453 = 1u32;
                                    fixed_u32::bit_and(a452, a453)
                                };
                                let a455 = 0u32;
                                if equal(a454, a455) {
                                    Bytes::lit(&[0x06])
                                } else {
                                    Bytes::lit(&[0x07])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0x08])
                                } else {
                                    Bytes::lit(&[0x09])
                                }
                            } else {
                                let a470 = {
                                    let a468 = v0;
                                    let a469 = 1u32;
                                    fixed_u32::bit_and(a468, a469)
                                };
                                let a471 = 0u32;
                                if equal(a470, a471) {
                                    Bytes::lit(&[0x0a])
                                } else {
                                    Bytes::lit(&[0x0b])
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
                                    Bytes::lit(&[0x0c])
                                } else {
                                    Bytes::lit(&[0x0d])
                                }
                            } else {
                                let a482 = {
                                    let a480 = v0;
                                    let a481 = 1u32;
                                    fixed_u32::bit_and(a480, a481)
                                };
                                let a483 = 0u32;
                                if equal(a482, a483) {
                                    Bytes::lit(&[0x0e])
                                } else {
                                    Bytes::lit(&[0x0f])
                                }
                            }
                        }
                    }
                } else {
                    let a486 = {
                        let a484 = v0;
                        let a485 = 8u32;
                        fixed_u32::bit_and(a484, a485)
                    };
                    let a487 = 0u32;
                    if equal(a486, a487) {
                        let a490 = {
                            let a488 = v0;
                            let a489 = 4u32;
                            fixed_u32::bit_and(a488, a489)
                        };
                        let a491 = 0u32;
                        if equal(a490, a491) {
                            let a494 = {
                                let a492 = v0;
                                let a493 = 2u32;
                                fixed_u32::bit_and(a492, a493)
                            };
                            let a495 = 0u32;
                            if equal(a494, a495) {
                                let a498 = {
                                    let a496 = v0;
                                    let a497 = 1u32;
                                    fixed_u32::bit_and(a496, a497)
                                };
                                let a499 = 0u32;
                                if equal(a498, a499) {
                                    Bytes::lit(&[0x10])
                                } else {
                                    Bytes::lit(&[0x11])
                                }
                            } else {
                                let a502 = {
                                    let a500 = v0;
                                    let a501 = 1u32;
                                    fixed_u32::bit_and(a500, a501)
                                };
                                let a503 = 0u32;
                                if equal(a502, a503) {
                                    Bytes::lit(&[0x12])
                                } else {
                                    Bytes::lit(&[0x13])
                                }
                            }
                        } else {
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
                                    Bytes::lit(&[0x14])
                                } else {
                                    Bytes::lit(&[0x15])
                                }
                            } else {
                                let a514 = {
                                    let a512 = v0;
                                    let a513 = 1u32;
                                    fixed_u32::bit_and(a512, a513)
                                };
                                let a515 = 0u32;
                                if equal(a514, a515) {
                                    Bytes::lit(&[0x16])
                                } else {
                                    Bytes::lit(&[0x17])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0x18])
                                } else {
                                    Bytes::lit(&[0x19])
                                }
                            } else {
                                let a530 = {
                                    let a528 = v0;
                                    let a529 = 1u32;
                                    fixed_u32::bit_and(a528, a529)
                                };
                                let a531 = 0u32;
                                if equal(a530, a531) {
                                    Bytes::lit(&[0x1a])
                                } else {
                                    Bytes::lit(&[0x1b])
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
                                    Bytes::lit(&[0x1c])
                                } else {
                                    Bytes::lit(&[0x1d])
                                }
                            } else {
                                let a542 = {
                                    let a540 = v0;
                                    let a541 = 1u32;
                                    fixed_u32::bit_and(a540, a541)
                                };
                                let a543 = 0u32;
                                if equal(a542, a543) {
                                    Bytes::lit(&[0x1e])
                                } else {
                                    Bytes::lit(&[0x1f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a546 = {
                    let a544 = v0;
                    let a545 = 16u32;
                    fixed_u32::bit_and(a544, a545)
                };
                let a547 = 0u32;
                if equal(a546, a547) {
                    let a550 = {
                        let a548 = v0;
                        let a549 = 8u32;
                        fixed_u32::bit_and(a548, a549)
                    };
                    let a551 = 0u32;
                    if equal(a550, a551) {
                        let a554 = {
                            let a552 = v0;
                            let a553 = 4u32;
                            fixed_u32::bit_and(a552, a553)
                        };
                        let a555 = 0u32;
                        if equal(a554, a555) {
                            let a558 = {
                                let a556 = v0;
                                let a557 = 2u32;
                                fixed_u32::bit_and(a556, a557)
                            };
                            let a559 = 0u32;
                            if equal(a558, a559) {
                                let a562 = {
                                    let a560 = v0;
                                    let a561 = 1u32;
                                    fixed_u32::bit_and(a560, a561)
                                };
                                let a563 = 0u32;
                                if equal(a562, a563) {
                                    Bytes::lit(&[0x20])
                                } else {
                                    Bytes::lit(&[0x21])
                                }
                            } else {
                                let a566 = {
                                    let a564 = v0;
                                    let a565 = 1u32;
                                    fixed_u32::bit_and(a564, a565)
                                };
                                let a567 = 0u32;
                                if equal(a566, a567) {
                                    Bytes::lit(&[0x22])
                                } else {
                                    Bytes::lit(&[0x23])
                                }
                            }
                        } else {
                            let a570 = {
                                let a568 = v0;
                                let a569 = 2u32;
                                fixed_u32::bit_and(a568, a569)
                            };
                            let a571 = 0u32;
                            if equal(a570, a571) {
                                let a574 = {
                                    let a572 = v0;
                                    let a573 = 1u32;
                                    fixed_u32::bit_and(a572, a573)
                                };
                                let a575 = 0u32;
                                if equal(a574, a575) {
                                    Bytes::lit(&[0x24])
                                } else {
                                    Bytes::lit(&[0x25])
                                }
                            } else {
                                let a578 = {
                                    let a576 = v0;
                                    let a577 = 1u32;
                                    fixed_u32::bit_and(a576, a577)
                                };
                                let a579 = 0u32;
                                if equal(a578, a579) {
                                    Bytes::lit(&[0x26])
                                } else {
                                    Bytes::lit(&[0x27])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0x28])
                                } else {
                                    Bytes::lit(&[0x29])
                                }
                            } else {
                                let a594 = {
                                    let a592 = v0;
                                    let a593 = 1u32;
                                    fixed_u32::bit_and(a592, a593)
                                };
                                let a595 = 0u32;
                                if equal(a594, a595) {
                                    Bytes::lit(&[0x2a])
                                } else {
                                    Bytes::lit(&[0x2b])
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
                                    Bytes::lit(&[0x2c])
                                } else {
                                    Bytes::lit(&[0x2d])
                                }
                            } else {
                                let a606 = {
                                    let a604 = v0;
                                    let a605 = 1u32;
                                    fixed_u32::bit_and(a604, a605)
                                };
                                let a607 = 0u32;
                                if equal(a606, a607) {
                                    Bytes::lit(&[0x2e])
                                } else {
                                    Bytes::lit(&[0x2f])
                                }
                            }
                        }
                    }
                } else {
                    let a610 = {
                        let a608 = v0;
                        let a609 = 8u32;
                        fixed_u32::bit_and(a608, a609)
                    };
                    let a611 = 0u32;
                    if equal(a610, a611) {
                        let a614 = {
                            let a612 = v0;
                            let a613 = 4u32;
                            fixed_u32::bit_and(a612, a613)
                        };
                        let a615 = 0u32;
                        if equal(a614, a615) {
                            let a618 = {
                                let a616 = v0;
                                let a617 = 2u32;
                                fixed_u32::bit_and(a616, a617)
                            };
                            let a619 = 0u32;
                            if equal(a618, a619) {
                                let a622 = {
                                    let a620 = v0;
                                    let a621 = 1u32;
                                    fixed_u32::bit_and(a620, a621)
                                };
                                let a623 = 0u32;
                                if equal(a622, a623) {
                                    Bytes::lit(&[0x30])
                                } else {
                                    Bytes::lit(&[0x31])
                                }
                            } else {
                                let a626 = {
                                    let a624 = v0;
                                    let a625 = 1u32;
                                    fixed_u32::bit_and(a624, a625)
                                };
                                let a627 = 0u32;
                                if equal(a626, a627) {
                                    Bytes::lit(&[0x32])
                                } else {
                                    Bytes::lit(&[0x33])
                                }
                            }
                        } else {
                            let a630 = {
                                let a628 = v0;
                                let a629 = 2u32;
                                fixed_u32::bit_and(a628, a629)
                            };
                            let a631 = 0u32;
                            if equal(a630, a631) {
                                let a634 = {
                                    let a632 = v0;
                                    let a633 = 1u32;
                                    fixed_u32::bit_and(a632, a633)
                                };
                                let a635 = 0u32;
                                if equal(a634, a635) {
                                    Bytes::lit(&[0x34])
                                } else {
                                    Bytes::lit(&[0x35])
                                }
                            } else {
                                let a638 = {
                                    let a636 = v0;
                                    let a637 = 1u32;
                                    fixed_u32::bit_and(a636, a637)
                                };
                                let a639 = 0u32;
                                if equal(a638, a639) {
                                    Bytes::lit(&[0x36])
                                } else {
                                    Bytes::lit(&[0x37])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0x38])
                                } else {
                                    Bytes::lit(&[0x39])
                                }
                            } else {
                                let a654 = {
                                    let a652 = v0;
                                    let a653 = 1u32;
                                    fixed_u32::bit_and(a652, a653)
                                };
                                let a655 = 0u32;
                                if equal(a654, a655) {
                                    Bytes::lit(&[0x3a])
                                } else {
                                    Bytes::lit(&[0x3b])
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
                                    Bytes::lit(&[0x3c])
                                } else {
                                    Bytes::lit(&[0x3d])
                                }
                            } else {
                                let a666 = {
                                    let a664 = v0;
                                    let a665 = 1u32;
                                    fixed_u32::bit_and(a664, a665)
                                };
                                let a667 = 0u32;
                                if equal(a666, a667) {
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
            let a670 = {
                let a668 = v0;
                let a669 = 32u32;
                fixed_u32::bit_and(a668, a669)
            };
            let a671 = 0u32;
            if equal(a670, a671) {
                let a674 = {
                    let a672 = v0;
                    let a673 = 16u32;
                    fixed_u32::bit_and(a672, a673)
                };
                let a675 = 0u32;
                if equal(a674, a675) {
                    let a678 = {
                        let a676 = v0;
                        let a677 = 8u32;
                        fixed_u32::bit_and(a676, a677)
                    };
                    let a679 = 0u32;
                    if equal(a678, a679) {
                        let a682 = {
                            let a680 = v0;
                            let a681 = 4u32;
                            fixed_u32::bit_and(a680, a681)
                        };
                        let a683 = 0u32;
                        if equal(a682, a683) {
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
                                    Bytes::lit(&[0x40])
                                } else {
                                    Bytes::lit(&[0x41])
                                }
                            } else {
                                let a694 = {
                                    let a692 = v0;
                                    let a693 = 1u32;
                                    fixed_u32::bit_and(a692, a693)
                                };
                                let a695 = 0u32;
                                if equal(a694, a695) {
                                    Bytes::lit(&[0x42])
                                } else {
                                    Bytes::lit(&[0x43])
                                }
                            }
                        } else {
                            let a698 = {
                                let a696 = v0;
                                let a697 = 2u32;
                                fixed_u32::bit_and(a696, a697)
                            };
                            let a699 = 0u32;
                            if equal(a698, a699) {
                                let a702 = {
                                    let a700 = v0;
                                    let a701 = 1u32;
                                    fixed_u32::bit_and(a700, a701)
                                };
                                let a703 = 0u32;
                                if equal(a702, a703) {
                                    Bytes::lit(&[0x44])
                                } else {
                                    Bytes::lit(&[0x45])
                                }
                            } else {
                                let a706 = {
                                    let a704 = v0;
                                    let a705 = 1u32;
                                    fixed_u32::bit_and(a704, a705)
                                };
                                let a707 = 0u32;
                                if equal(a706, a707) {
                                    Bytes::lit(&[0x46])
                                } else {
                                    Bytes::lit(&[0x47])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0x48])
                                } else {
                                    Bytes::lit(&[0x49])
                                }
                            } else {
                                let a722 = {
                                    let a720 = v0;
                                    let a721 = 1u32;
                                    fixed_u32::bit_and(a720, a721)
                                };
                                let a723 = 0u32;
                                if equal(a722, a723) {
                                    Bytes::lit(&[0x4a])
                                } else {
                                    Bytes::lit(&[0x4b])
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
                                    Bytes::lit(&[0x4c])
                                } else {
                                    Bytes::lit(&[0x4d])
                                }
                            } else {
                                let a734 = {
                                    let a732 = v0;
                                    let a733 = 1u32;
                                    fixed_u32::bit_and(a732, a733)
                                };
                                let a735 = 0u32;
                                if equal(a734, a735) {
                                    Bytes::lit(&[0x4e])
                                } else {
                                    Bytes::lit(&[0x4f])
                                }
                            }
                        }
                    }
                } else {
                    let a738 = {
                        let a736 = v0;
                        let a737 = 8u32;
                        fixed_u32::bit_and(a736, a737)
                    };
                    let a739 = 0u32;
                    if equal(a738, a739) {
                        let a742 = {
                            let a740 = v0;
                            let a741 = 4u32;
                            fixed_u32::bit_and(a740, a741)
                        };
                        let a743 = 0u32;
                        if equal(a742, a743) {
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
                                    Bytes::lit(&[0x50])
                                } else {
                                    Bytes::lit(&[0x51])
                                }
                            } else {
                                let a754 = {
                                    let a752 = v0;
                                    let a753 = 1u32;
                                    fixed_u32::bit_and(a752, a753)
                                };
                                let a755 = 0u32;
                                if equal(a754, a755) {
                                    Bytes::lit(&[0x52])
                                } else {
                                    Bytes::lit(&[0x53])
                                }
                            }
                        } else {
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
                                    Bytes::lit(&[0x54])
                                } else {
                                    Bytes::lit(&[0x55])
                                }
                            } else {
                                let a766 = {
                                    let a764 = v0;
                                    let a765 = 1u32;
                                    fixed_u32::bit_and(a764, a765)
                                };
                                let a767 = 0u32;
                                if equal(a766, a767) {
                                    Bytes::lit(&[0x56])
                                } else {
                                    Bytes::lit(&[0x57])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0x58])
                                } else {
                                    Bytes::lit(&[0x59])
                                }
                            } else {
                                let a782 = {
                                    let a780 = v0;
                                    let a781 = 1u32;
                                    fixed_u32::bit_and(a780, a781)
                                };
                                let a783 = 0u32;
                                if equal(a782, a783) {
                                    Bytes::lit(&[0x5a])
                                } else {
                                    Bytes::lit(&[0x5b])
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
                                    Bytes::lit(&[0x5c])
                                } else {
                                    Bytes::lit(&[0x5d])
                                }
                            } else {
                                let a794 = {
                                    let a792 = v0;
                                    let a793 = 1u32;
                                    fixed_u32::bit_and(a792, a793)
                                };
                                let a795 = 0u32;
                                if equal(a794, a795) {
                                    Bytes::lit(&[0x5e])
                                } else {
                                    Bytes::lit(&[0x5f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a798 = {
                    let a796 = v0;
                    let a797 = 16u32;
                    fixed_u32::bit_and(a796, a797)
                };
                let a799 = 0u32;
                if equal(a798, a799) {
                    let a802 = {
                        let a800 = v0;
                        let a801 = 8u32;
                        fixed_u32::bit_and(a800, a801)
                    };
                    let a803 = 0u32;
                    if equal(a802, a803) {
                        let a806 = {
                            let a804 = v0;
                            let a805 = 4u32;
                            fixed_u32::bit_and(a804, a805)
                        };
                        let a807 = 0u32;
                        if equal(a806, a807) {
                            let a810 = {
                                let a808 = v0;
                                let a809 = 2u32;
                                fixed_u32::bit_and(a808, a809)
                            };
                            let a811 = 0u32;
                            if equal(a810, a811) {
                                let a814 = {
                                    let a812 = v0;
                                    let a813 = 1u32;
                                    fixed_u32::bit_and(a812, a813)
                                };
                                let a815 = 0u32;
                                if equal(a814, a815) {
                                    Bytes::lit(&[0x60])
                                } else {
                                    Bytes::lit(&[0x61])
                                }
                            } else {
                                let a818 = {
                                    let a816 = v0;
                                    let a817 = 1u32;
                                    fixed_u32::bit_and(a816, a817)
                                };
                                let a819 = 0u32;
                                if equal(a818, a819) {
                                    Bytes::lit(&[0x62])
                                } else {
                                    Bytes::lit(&[0x63])
                                }
                            }
                        } else {
                            let a822 = {
                                let a820 = v0;
                                let a821 = 2u32;
                                fixed_u32::bit_and(a820, a821)
                            };
                            let a823 = 0u32;
                            if equal(a822, a823) {
                                let a826 = {
                                    let a824 = v0;
                                    let a825 = 1u32;
                                    fixed_u32::bit_and(a824, a825)
                                };
                                let a827 = 0u32;
                                if equal(a826, a827) {
                                    Bytes::lit(&[0x64])
                                } else {
                                    Bytes::lit(&[0x65])
                                }
                            } else {
                                let a830 = {
                                    let a828 = v0;
                                    let a829 = 1u32;
                                    fixed_u32::bit_and(a828, a829)
                                };
                                let a831 = 0u32;
                                if equal(a830, a831) {
                                    Bytes::lit(&[0x66])
                                } else {
                                    Bytes::lit(&[0x67])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0x68])
                                } else {
                                    Bytes::lit(&[0x69])
                                }
                            } else {
                                let a846 = {
                                    let a844 = v0;
                                    let a845 = 1u32;
                                    fixed_u32::bit_and(a844, a845)
                                };
                                let a847 = 0u32;
                                if equal(a846, a847) {
                                    Bytes::lit(&[0x6a])
                                } else {
                                    Bytes::lit(&[0x6b])
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
                                    Bytes::lit(&[0x6c])
                                } else {
                                    Bytes::lit(&[0x6d])
                                }
                            } else {
                                let a858 = {
                                    let a856 = v0;
                                    let a857 = 1u32;
                                    fixed_u32::bit_and(a856, a857)
                                };
                                let a859 = 0u32;
                                if equal(a858, a859) {
                                    Bytes::lit(&[0x6e])
                                } else {
                                    Bytes::lit(&[0x6f])
                                }
                            }
                        }
                    }
                } else {
                    let a862 = {
                        let a860 = v0;
                        let a861 = 8u32;
                        fixed_u32::bit_and(a860, a861)
                    };
                    let a863 = 0u32;
                    if equal(a862, a863) {
                        let a866 = {
                            let a864 = v0;
                            let a865 = 4u32;
                            fixed_u32::bit_and(a864, a865)
                        };
                        let a867 = 0u32;
                        if equal(a866, a867) {
                            let a870 = {
                                let a868 = v0;
                                let a869 = 2u32;
                                fixed_u32::bit_and(a868, a869)
                            };
                            let a871 = 0u32;
                            if equal(a870, a871) {
                                let a874 = {
                                    let a872 = v0;
                                    let a873 = 1u32;
                                    fixed_u32::bit_and(a872, a873)
                                };
                                let a875 = 0u32;
                                if equal(a874, a875) {
                                    Bytes::lit(&[0x70])
                                } else {
                                    Bytes::lit(&[0x71])
                                }
                            } else {
                                let a878 = {
                                    let a876 = v0;
                                    let a877 = 1u32;
                                    fixed_u32::bit_and(a876, a877)
                                };
                                let a879 = 0u32;
                                if equal(a878, a879) {
                                    Bytes::lit(&[0x72])
                                } else {
                                    Bytes::lit(&[0x73])
                                }
                            }
                        } else {
                            let a882 = {
                                let a880 = v0;
                                let a881 = 2u32;
                                fixed_u32::bit_and(a880, a881)
                            };
                            let a883 = 0u32;
                            if equal(a882, a883) {
                                let a886 = {
                                    let a884 = v0;
                                    let a885 = 1u32;
                                    fixed_u32::bit_and(a884, a885)
                                };
                                let a887 = 0u32;
                                if equal(a886, a887) {
                                    Bytes::lit(&[0x74])
                                } else {
                                    Bytes::lit(&[0x75])
                                }
                            } else {
                                let a890 = {
                                    let a888 = v0;
                                    let a889 = 1u32;
                                    fixed_u32::bit_and(a888, a889)
                                };
                                let a891 = 0u32;
                                if equal(a890, a891) {
                                    Bytes::lit(&[0x76])
                                } else {
                                    Bytes::lit(&[0x77])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0x78])
                                } else {
                                    Bytes::lit(&[0x79])
                                }
                            } else {
                                let a906 = {
                                    let a904 = v0;
                                    let a905 = 1u32;
                                    fixed_u32::bit_and(a904, a905)
                                };
                                let a907 = 0u32;
                                if equal(a906, a907) {
                                    Bytes::lit(&[0x7a])
                                } else {
                                    Bytes::lit(&[0x7b])
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
                                    Bytes::lit(&[0x7c])
                                } else {
                                    Bytes::lit(&[0x7d])
                                }
                            } else {
                                let a918 = {
                                    let a916 = v0;
                                    let a917 = 1u32;
                                    fixed_u32::bit_and(a916, a917)
                                };
                                let a919 = 0u32;
                                if equal(a918, a919) {
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
        let a922 = {
            let a920 = v0;
            let a921 = 64u32;
            fixed_u32::bit_and(a920, a921)
        };
        let a923 = 0u32;
        if equal(a922, a923) {
            let a926 = {
                let a924 = v0;
                let a925 = 32u32;
                fixed_u32::bit_and(a924, a925)
            };
            let a927 = 0u32;
            if equal(a926, a927) {
                let a930 = {
                    let a928 = v0;
                    let a929 = 16u32;
                    fixed_u32::bit_and(a928, a929)
                };
                let a931 = 0u32;
                if equal(a930, a931) {
                    let a934 = {
                        let a932 = v0;
                        let a933 = 8u32;
                        fixed_u32::bit_and(a932, a933)
                    };
                    let a935 = 0u32;
                    if equal(a934, a935) {
                        let a938 = {
                            let a936 = v0;
                            let a937 = 4u32;
                            fixed_u32::bit_and(a936, a937)
                        };
                        let a939 = 0u32;
                        if equal(a938, a939) {
                            let a942 = {
                                let a940 = v0;
                                let a941 = 2u32;
                                fixed_u32::bit_and(a940, a941)
                            };
                            let a943 = 0u32;
                            if equal(a942, a943) {
                                let a946 = {
                                    let a944 = v0;
                                    let a945 = 1u32;
                                    fixed_u32::bit_and(a944, a945)
                                };
                                let a947 = 0u32;
                                if equal(a946, a947) {
                                    Bytes::lit(&[0x80])
                                } else {
                                    Bytes::lit(&[0x81])
                                }
                            } else {
                                let a950 = {
                                    let a948 = v0;
                                    let a949 = 1u32;
                                    fixed_u32::bit_and(a948, a949)
                                };
                                let a951 = 0u32;
                                if equal(a950, a951) {
                                    Bytes::lit(&[0x82])
                                } else {
                                    Bytes::lit(&[0x83])
                                }
                            }
                        } else {
                            let a954 = {
                                let a952 = v0;
                                let a953 = 2u32;
                                fixed_u32::bit_and(a952, a953)
                            };
                            let a955 = 0u32;
                            if equal(a954, a955) {
                                let a958 = {
                                    let a956 = v0;
                                    let a957 = 1u32;
                                    fixed_u32::bit_and(a956, a957)
                                };
                                let a959 = 0u32;
                                if equal(a958, a959) {
                                    Bytes::lit(&[0x84])
                                } else {
                                    Bytes::lit(&[0x85])
                                }
                            } else {
                                let a962 = {
                                    let a960 = v0;
                                    let a961 = 1u32;
                                    fixed_u32::bit_and(a960, a961)
                                };
                                let a963 = 0u32;
                                if equal(a962, a963) {
                                    Bytes::lit(&[0x86])
                                } else {
                                    Bytes::lit(&[0x87])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0x88])
                                } else {
                                    Bytes::lit(&[0x89])
                                }
                            } else {
                                let a978 = {
                                    let a976 = v0;
                                    let a977 = 1u32;
                                    fixed_u32::bit_and(a976, a977)
                                };
                                let a979 = 0u32;
                                if equal(a978, a979) {
                                    Bytes::lit(&[0x8a])
                                } else {
                                    Bytes::lit(&[0x8b])
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
                                    Bytes::lit(&[0x8c])
                                } else {
                                    Bytes::lit(&[0x8d])
                                }
                            } else {
                                let a990 = {
                                    let a988 = v0;
                                    let a989 = 1u32;
                                    fixed_u32::bit_and(a988, a989)
                                };
                                let a991 = 0u32;
                                if equal(a990, a991) {
                                    Bytes::lit(&[0x8e])
                                } else {
                                    Bytes::lit(&[0x8f])
                                }
                            }
                        }
                    }
                } else {
                    let a994 = {
                        let a992 = v0;
                        let a993 = 8u32;
                        fixed_u32::bit_and(a992, a993)
                    };
                    let a995 = 0u32;
                    if equal(a994, a995) {
                        let a998 = {
                            let a996 = v0;
                            let a997 = 4u32;
                            fixed_u32::bit_and(a996, a997)
                        };
                        let a999 = 0u32;
                        if equal(a998, a999) {
                            let a1002 = {
                                let a1000 = v0;
                                let a1001 = 2u32;
                                fixed_u32::bit_and(a1000, a1001)
                            };
                            let a1003 = 0u32;
                            if equal(a1002, a1003) {
                                let a1006 = {
                                    let a1004 = v0;
                                    let a1005 = 1u32;
                                    fixed_u32::bit_and(a1004, a1005)
                                };
                                let a1007 = 0u32;
                                if equal(a1006, a1007) {
                                    Bytes::lit(&[0x90])
                                } else {
                                    Bytes::lit(&[0x91])
                                }
                            } else {
                                let a1010 = {
                                    let a1008 = v0;
                                    let a1009 = 1u32;
                                    fixed_u32::bit_and(a1008, a1009)
                                };
                                let a1011 = 0u32;
                                if equal(a1010, a1011) {
                                    Bytes::lit(&[0x92])
                                } else {
                                    Bytes::lit(&[0x93])
                                }
                            }
                        } else {
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
                                    Bytes::lit(&[0x94])
                                } else {
                                    Bytes::lit(&[0x95])
                                }
                            } else {
                                let a1022 = {
                                    let a1020 = v0;
                                    let a1021 = 1u32;
                                    fixed_u32::bit_and(a1020, a1021)
                                };
                                let a1023 = 0u32;
                                if equal(a1022, a1023) {
                                    Bytes::lit(&[0x96])
                                } else {
                                    Bytes::lit(&[0x97])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0x98])
                                } else {
                                    Bytes::lit(&[0x99])
                                }
                            } else {
                                let a1038 = {
                                    let a1036 = v0;
                                    let a1037 = 1u32;
                                    fixed_u32::bit_and(a1036, a1037)
                                };
                                let a1039 = 0u32;
                                if equal(a1038, a1039) {
                                    Bytes::lit(&[0x9a])
                                } else {
                                    Bytes::lit(&[0x9b])
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
                                    Bytes::lit(&[0x9c])
                                } else {
                                    Bytes::lit(&[0x9d])
                                }
                            } else {
                                let a1050 = {
                                    let a1048 = v0;
                                    let a1049 = 1u32;
                                    fixed_u32::bit_and(a1048, a1049)
                                };
                                let a1051 = 0u32;
                                if equal(a1050, a1051) {
                                    Bytes::lit(&[0x9e])
                                } else {
                                    Bytes::lit(&[0x9f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a1054 = {
                    let a1052 = v0;
                    let a1053 = 16u32;
                    fixed_u32::bit_and(a1052, a1053)
                };
                let a1055 = 0u32;
                if equal(a1054, a1055) {
                    let a1058 = {
                        let a1056 = v0;
                        let a1057 = 8u32;
                        fixed_u32::bit_and(a1056, a1057)
                    };
                    let a1059 = 0u32;
                    if equal(a1058, a1059) {
                        let a1062 = {
                            let a1060 = v0;
                            let a1061 = 4u32;
                            fixed_u32::bit_and(a1060, a1061)
                        };
                        let a1063 = 0u32;
                        if equal(a1062, a1063) {
                            let a1066 = {
                                let a1064 = v0;
                                let a1065 = 2u32;
                                fixed_u32::bit_and(a1064, a1065)
                            };
                            let a1067 = 0u32;
                            if equal(a1066, a1067) {
                                let a1070 = {
                                    let a1068 = v0;
                                    let a1069 = 1u32;
                                    fixed_u32::bit_and(a1068, a1069)
                                };
                                let a1071 = 0u32;
                                if equal(a1070, a1071) {
                                    Bytes::lit(&[0xa0])
                                } else {
                                    Bytes::lit(&[0xa1])
                                }
                            } else {
                                let a1074 = {
                                    let a1072 = v0;
                                    let a1073 = 1u32;
                                    fixed_u32::bit_and(a1072, a1073)
                                };
                                let a1075 = 0u32;
                                if equal(a1074, a1075) {
                                    Bytes::lit(&[0xa2])
                                } else {
                                    Bytes::lit(&[0xa3])
                                }
                            }
                        } else {
                            let a1078 = {
                                let a1076 = v0;
                                let a1077 = 2u32;
                                fixed_u32::bit_and(a1076, a1077)
                            };
                            let a1079 = 0u32;
                            if equal(a1078, a1079) {
                                let a1082 = {
                                    let a1080 = v0;
                                    let a1081 = 1u32;
                                    fixed_u32::bit_and(a1080, a1081)
                                };
                                let a1083 = 0u32;
                                if equal(a1082, a1083) {
                                    Bytes::lit(&[0xa4])
                                } else {
                                    Bytes::lit(&[0xa5])
                                }
                            } else {
                                let a1086 = {
                                    let a1084 = v0;
                                    let a1085 = 1u32;
                                    fixed_u32::bit_and(a1084, a1085)
                                };
                                let a1087 = 0u32;
                                if equal(a1086, a1087) {
                                    Bytes::lit(&[0xa6])
                                } else {
                                    Bytes::lit(&[0xa7])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0xa8])
                                } else {
                                    Bytes::lit(&[0xa9])
                                }
                            } else {
                                let a1102 = {
                                    let a1100 = v0;
                                    let a1101 = 1u32;
                                    fixed_u32::bit_and(a1100, a1101)
                                };
                                let a1103 = 0u32;
                                if equal(a1102, a1103) {
                                    Bytes::lit(&[0xaa])
                                } else {
                                    Bytes::lit(&[0xab])
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
                                    Bytes::lit(&[0xac])
                                } else {
                                    Bytes::lit(&[0xad])
                                }
                            } else {
                                let a1114 = {
                                    let a1112 = v0;
                                    let a1113 = 1u32;
                                    fixed_u32::bit_and(a1112, a1113)
                                };
                                let a1115 = 0u32;
                                if equal(a1114, a1115) {
                                    Bytes::lit(&[0xae])
                                } else {
                                    Bytes::lit(&[0xaf])
                                }
                            }
                        }
                    }
                } else {
                    let a1118 = {
                        let a1116 = v0;
                        let a1117 = 8u32;
                        fixed_u32::bit_and(a1116, a1117)
                    };
                    let a1119 = 0u32;
                    if equal(a1118, a1119) {
                        let a1122 = {
                            let a1120 = v0;
                            let a1121 = 4u32;
                            fixed_u32::bit_and(a1120, a1121)
                        };
                        let a1123 = 0u32;
                        if equal(a1122, a1123) {
                            let a1126 = {
                                let a1124 = v0;
                                let a1125 = 2u32;
                                fixed_u32::bit_and(a1124, a1125)
                            };
                            let a1127 = 0u32;
                            if equal(a1126, a1127) {
                                let a1130 = {
                                    let a1128 = v0;
                                    let a1129 = 1u32;
                                    fixed_u32::bit_and(a1128, a1129)
                                };
                                let a1131 = 0u32;
                                if equal(a1130, a1131) {
                                    Bytes::lit(&[0xb0])
                                } else {
                                    Bytes::lit(&[0xb1])
                                }
                            } else {
                                let a1134 = {
                                    let a1132 = v0;
                                    let a1133 = 1u32;
                                    fixed_u32::bit_and(a1132, a1133)
                                };
                                let a1135 = 0u32;
                                if equal(a1134, a1135) {
                                    Bytes::lit(&[0xb2])
                                } else {
                                    Bytes::lit(&[0xb3])
                                }
                            }
                        } else {
                            let a1138 = {
                                let a1136 = v0;
                                let a1137 = 2u32;
                                fixed_u32::bit_and(a1136, a1137)
                            };
                            let a1139 = 0u32;
                            if equal(a1138, a1139) {
                                let a1142 = {
                                    let a1140 = v0;
                                    let a1141 = 1u32;
                                    fixed_u32::bit_and(a1140, a1141)
                                };
                                let a1143 = 0u32;
                                if equal(a1142, a1143) {
                                    Bytes::lit(&[0xb4])
                                } else {
                                    Bytes::lit(&[0xb5])
                                }
                            } else {
                                let a1146 = {
                                    let a1144 = v0;
                                    let a1145 = 1u32;
                                    fixed_u32::bit_and(a1144, a1145)
                                };
                                let a1147 = 0u32;
                                if equal(a1146, a1147) {
                                    Bytes::lit(&[0xb6])
                                } else {
                                    Bytes::lit(&[0xb7])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0xb8])
                                } else {
                                    Bytes::lit(&[0xb9])
                                }
                            } else {
                                let a1162 = {
                                    let a1160 = v0;
                                    let a1161 = 1u32;
                                    fixed_u32::bit_and(a1160, a1161)
                                };
                                let a1163 = 0u32;
                                if equal(a1162, a1163) {
                                    Bytes::lit(&[0xba])
                                } else {
                                    Bytes::lit(&[0xbb])
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
                                    Bytes::lit(&[0xbc])
                                } else {
                                    Bytes::lit(&[0xbd])
                                }
                            } else {
                                let a1174 = {
                                    let a1172 = v0;
                                    let a1173 = 1u32;
                                    fixed_u32::bit_and(a1172, a1173)
                                };
                                let a1175 = 0u32;
                                if equal(a1174, a1175) {
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
            let a1178 = {
                let a1176 = v0;
                let a1177 = 32u32;
                fixed_u32::bit_and(a1176, a1177)
            };
            let a1179 = 0u32;
            if equal(a1178, a1179) {
                let a1182 = {
                    let a1180 = v0;
                    let a1181 = 16u32;
                    fixed_u32::bit_and(a1180, a1181)
                };
                let a1183 = 0u32;
                if equal(a1182, a1183) {
                    let a1186 = {
                        let a1184 = v0;
                        let a1185 = 8u32;
                        fixed_u32::bit_and(a1184, a1185)
                    };
                    let a1187 = 0u32;
                    if equal(a1186, a1187) {
                        let a1190 = {
                            let a1188 = v0;
                            let a1189 = 4u32;
                            fixed_u32::bit_and(a1188, a1189)
                        };
                        let a1191 = 0u32;
                        if equal(a1190, a1191) {
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
                                    Bytes::lit(&[0xc0])
                                } else {
                                    Bytes::lit(&[0xc1])
                                }
                            } else {
                                let a1202 = {
                                    let a1200 = v0;
                                    let a1201 = 1u32;
                                    fixed_u32::bit_and(a1200, a1201)
                                };
                                let a1203 = 0u32;
                                if equal(a1202, a1203) {
                                    Bytes::lit(&[0xc2])
                                } else {
                                    Bytes::lit(&[0xc3])
                                }
                            }
                        } else {
                            let a1206 = {
                                let a1204 = v0;
                                let a1205 = 2u32;
                                fixed_u32::bit_and(a1204, a1205)
                            };
                            let a1207 = 0u32;
                            if equal(a1206, a1207) {
                                let a1210 = {
                                    let a1208 = v0;
                                    let a1209 = 1u32;
                                    fixed_u32::bit_and(a1208, a1209)
                                };
                                let a1211 = 0u32;
                                if equal(a1210, a1211) {
                                    Bytes::lit(&[0xc4])
                                } else {
                                    Bytes::lit(&[0xc5])
                                }
                            } else {
                                let a1214 = {
                                    let a1212 = v0;
                                    let a1213 = 1u32;
                                    fixed_u32::bit_and(a1212, a1213)
                                };
                                let a1215 = 0u32;
                                if equal(a1214, a1215) {
                                    Bytes::lit(&[0xc6])
                                } else {
                                    Bytes::lit(&[0xc7])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0xc8])
                                } else {
                                    Bytes::lit(&[0xc9])
                                }
                            } else {
                                let a1230 = {
                                    let a1228 = v0;
                                    let a1229 = 1u32;
                                    fixed_u32::bit_and(a1228, a1229)
                                };
                                let a1231 = 0u32;
                                if equal(a1230, a1231) {
                                    Bytes::lit(&[0xca])
                                } else {
                                    Bytes::lit(&[0xcb])
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
                                    Bytes::lit(&[0xcc])
                                } else {
                                    Bytes::lit(&[0xcd])
                                }
                            } else {
                                let a1242 = {
                                    let a1240 = v0;
                                    let a1241 = 1u32;
                                    fixed_u32::bit_and(a1240, a1241)
                                };
                                let a1243 = 0u32;
                                if equal(a1242, a1243) {
                                    Bytes::lit(&[0xce])
                                } else {
                                    Bytes::lit(&[0xcf])
                                }
                            }
                        }
                    }
                } else {
                    let a1246 = {
                        let a1244 = v0;
                        let a1245 = 8u32;
                        fixed_u32::bit_and(a1244, a1245)
                    };
                    let a1247 = 0u32;
                    if equal(a1246, a1247) {
                        let a1250 = {
                            let a1248 = v0;
                            let a1249 = 4u32;
                            fixed_u32::bit_and(a1248, a1249)
                        };
                        let a1251 = 0u32;
                        if equal(a1250, a1251) {
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
                                    Bytes::lit(&[0xd0])
                                } else {
                                    Bytes::lit(&[0xd1])
                                }
                            } else {
                                let a1262 = {
                                    let a1260 = v0;
                                    let a1261 = 1u32;
                                    fixed_u32::bit_and(a1260, a1261)
                                };
                                let a1263 = 0u32;
                                if equal(a1262, a1263) {
                                    Bytes::lit(&[0xd2])
                                } else {
                                    Bytes::lit(&[0xd3])
                                }
                            }
                        } else {
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
                                    Bytes::lit(&[0xd4])
                                } else {
                                    Bytes::lit(&[0xd5])
                                }
                            } else {
                                let a1274 = {
                                    let a1272 = v0;
                                    let a1273 = 1u32;
                                    fixed_u32::bit_and(a1272, a1273)
                                };
                                let a1275 = 0u32;
                                if equal(a1274, a1275) {
                                    Bytes::lit(&[0xd6])
                                } else {
                                    Bytes::lit(&[0xd7])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0xd8])
                                } else {
                                    Bytes::lit(&[0xd9])
                                }
                            } else {
                                let a1290 = {
                                    let a1288 = v0;
                                    let a1289 = 1u32;
                                    fixed_u32::bit_and(a1288, a1289)
                                };
                                let a1291 = 0u32;
                                if equal(a1290, a1291) {
                                    Bytes::lit(&[0xda])
                                } else {
                                    Bytes::lit(&[0xdb])
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
                                    Bytes::lit(&[0xdc])
                                } else {
                                    Bytes::lit(&[0xdd])
                                }
                            } else {
                                let a1302 = {
                                    let a1300 = v0;
                                    let a1301 = 1u32;
                                    fixed_u32::bit_and(a1300, a1301)
                                };
                                let a1303 = 0u32;
                                if equal(a1302, a1303) {
                                    Bytes::lit(&[0xde])
                                } else {
                                    Bytes::lit(&[0xdf])
                                }
                            }
                        }
                    }
                }
            } else {
                let a1306 = {
                    let a1304 = v0;
                    let a1305 = 16u32;
                    fixed_u32::bit_and(a1304, a1305)
                };
                let a1307 = 0u32;
                if equal(a1306, a1307) {
                    let a1310 = {
                        let a1308 = v0;
                        let a1309 = 8u32;
                        fixed_u32::bit_and(a1308, a1309)
                    };
                    let a1311 = 0u32;
                    if equal(a1310, a1311) {
                        let a1314 = {
                            let a1312 = v0;
                            let a1313 = 4u32;
                            fixed_u32::bit_and(a1312, a1313)
                        };
                        let a1315 = 0u32;
                        if equal(a1314, a1315) {
                            let a1318 = {
                                let a1316 = v0;
                                let a1317 = 2u32;
                                fixed_u32::bit_and(a1316, a1317)
                            };
                            let a1319 = 0u32;
                            if equal(a1318, a1319) {
                                let a1322 = {
                                    let a1320 = v0;
                                    let a1321 = 1u32;
                                    fixed_u32::bit_and(a1320, a1321)
                                };
                                let a1323 = 0u32;
                                if equal(a1322, a1323) {
                                    Bytes::lit(&[0xe0])
                                } else {
                                    Bytes::lit(&[0xe1])
                                }
                            } else {
                                let a1326 = {
                                    let a1324 = v0;
                                    let a1325 = 1u32;
                                    fixed_u32::bit_and(a1324, a1325)
                                };
                                let a1327 = 0u32;
                                if equal(a1326, a1327) {
                                    Bytes::lit(&[0xe2])
                                } else {
                                    Bytes::lit(&[0xe3])
                                }
                            }
                        } else {
                            let a1330 = {
                                let a1328 = v0;
                                let a1329 = 2u32;
                                fixed_u32::bit_and(a1328, a1329)
                            };
                            let a1331 = 0u32;
                            if equal(a1330, a1331) {
                                let a1334 = {
                                    let a1332 = v0;
                                    let a1333 = 1u32;
                                    fixed_u32::bit_and(a1332, a1333)
                                };
                                let a1335 = 0u32;
                                if equal(a1334, a1335) {
                                    Bytes::lit(&[0xe4])
                                } else {
                                    Bytes::lit(&[0xe5])
                                }
                            } else {
                                let a1338 = {
                                    let a1336 = v0;
                                    let a1337 = 1u32;
                                    fixed_u32::bit_and(a1336, a1337)
                                };
                                let a1339 = 0u32;
                                if equal(a1338, a1339) {
                                    Bytes::lit(&[0xe6])
                                } else {
                                    Bytes::lit(&[0xe7])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0xe8])
                                } else {
                                    Bytes::lit(&[0xe9])
                                }
                            } else {
                                let a1354 = {
                                    let a1352 = v0;
                                    let a1353 = 1u32;
                                    fixed_u32::bit_and(a1352, a1353)
                                };
                                let a1355 = 0u32;
                                if equal(a1354, a1355) {
                                    Bytes::lit(&[0xea])
                                } else {
                                    Bytes::lit(&[0xeb])
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
                                    Bytes::lit(&[0xec])
                                } else {
                                    Bytes::lit(&[0xed])
                                }
                            } else {
                                let a1366 = {
                                    let a1364 = v0;
                                    let a1365 = 1u32;
                                    fixed_u32::bit_and(a1364, a1365)
                                };
                                let a1367 = 0u32;
                                if equal(a1366, a1367) {
                                    Bytes::lit(&[0xee])
                                } else {
                                    Bytes::lit(&[0xef])
                                }
                            }
                        }
                    }
                } else {
                    let a1370 = {
                        let a1368 = v0;
                        let a1369 = 8u32;
                        fixed_u32::bit_and(a1368, a1369)
                    };
                    let a1371 = 0u32;
                    if equal(a1370, a1371) {
                        let a1374 = {
                            let a1372 = v0;
                            let a1373 = 4u32;
                            fixed_u32::bit_and(a1372, a1373)
                        };
                        let a1375 = 0u32;
                        if equal(a1374, a1375) {
                            let a1378 = {
                                let a1376 = v0;
                                let a1377 = 2u32;
                                fixed_u32::bit_and(a1376, a1377)
                            };
                            let a1379 = 0u32;
                            if equal(a1378, a1379) {
                                let a1382 = {
                                    let a1380 = v0;
                                    let a1381 = 1u32;
                                    fixed_u32::bit_and(a1380, a1381)
                                };
                                let a1383 = 0u32;
                                if equal(a1382, a1383) {
                                    Bytes::lit(&[0xf0])
                                } else {
                                    Bytes::lit(&[0xf1])
                                }
                            } else {
                                let a1386 = {
                                    let a1384 = v0;
                                    let a1385 = 1u32;
                                    fixed_u32::bit_and(a1384, a1385)
                                };
                                let a1387 = 0u32;
                                if equal(a1386, a1387) {
                                    Bytes::lit(&[0xf2])
                                } else {
                                    Bytes::lit(&[0xf3])
                                }
                            }
                        } else {
                            let a1390 = {
                                let a1388 = v0;
                                let a1389 = 2u32;
                                fixed_u32::bit_and(a1388, a1389)
                            };
                            let a1391 = 0u32;
                            if equal(a1390, a1391) {
                                let a1394 = {
                                    let a1392 = v0;
                                    let a1393 = 1u32;
                                    fixed_u32::bit_and(a1392, a1393)
                                };
                                let a1395 = 0u32;
                                if equal(a1394, a1395) {
                                    Bytes::lit(&[0xf4])
                                } else {
                                    Bytes::lit(&[0xf5])
                                }
                            } else {
                                let a1398 = {
                                    let a1396 = v0;
                                    let a1397 = 1u32;
                                    fixed_u32::bit_and(a1396, a1397)
                                };
                                let a1399 = 0u32;
                                if equal(a1398, a1399) {
                                    Bytes::lit(&[0xf6])
                                } else {
                                    Bytes::lit(&[0xf7])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0xf8])
                                } else {
                                    Bytes::lit(&[0xf9])
                                }
                            } else {
                                let a1414 = {
                                    let a1412 = v0;
                                    let a1413 = 1u32;
                                    fixed_u32::bit_and(a1412, a1413)
                                };
                                let a1415 = 0u32;
                                if equal(a1414, a1415) {
                                    Bytes::lit(&[0xfa])
                                } else {
                                    Bytes::lit(&[0xfb])
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
                                    Bytes::lit(&[0xfc])
                                } else {
                                    Bytes::lit(&[0xfd])
                                }
                            } else {
                                let a1426 = {
                                    let a1424 = v0;
                                    let a1425 = 1u32;
                                    fixed_u32::bit_and(a1424, a1425)
                                };
                                let a1427 = 0u32;
                                if equal(a1426, a1427) {
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

pub fn f52(v0: u32, v1: u32) -> u32 {
    let m1430 = {
        let a1428 = v0;
        let a1429 = v1;
        fixed_u32::shift_right(a1428, a1429)
    };
    match m1430 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f53(v0: Fn9, p1434: (Option<Bytes>, List<Bytes>), p1435: List<Bytes>) -> (Option<Bytes>, List<Bytes>) {
    let m1433 = (p1434, p1435);
    {
        let mut m1433 = m1433;
        loop {
            let step = {
                let (v1, v2) = m1433;
                let m1431 = v2;
                match m1431.uncons() {
                    None => {
                        Err::<((Option<Bytes>, List<Bytes>), List<Bytes>), (Option<Bytes>, List<Bytes>)>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<((Option<Bytes>, List<Bytes>), List<Bytes>), (Option<Bytes>, List<Bytes>)>(({
                            let c1432: Fn9 = v0.clone();
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

pub fn f54(v0: (Option<Bytes>, List<Bytes>), v1: Bytes) -> (Option<Bytes>, List<Bytes>) {
    let m1437 = {
        let (h1436, _) = v0;
        h1436
    };
    match m1437 {
        None => {
            (Some(v1), {
                let (_, h1441) = v0;
                h1441
            })
        }
        Some(v2) => {
            (None::<Bytes>, List::cons({
                let a1438 = v2;
                let a1439 = v1;
                append_bytes(a1438, a1439)
            }, {
                let (_, h1440) = v0;
                h1440
            }))
        }
    }
}

pub fn f55(v0: List<Bytes>, v1: Bytes) -> List<Bytes> {
    List::cons(v1, v0)
}

pub fn f56(v0: Bytes, v1: u64, v2: u64) -> R<u32> {
    Ok(f62(f63(v0.clone(), v1, v2), f63(v0.clone(), v1, {
        let a1442 = v2;
        let a1443 = 1u64;
        nat_add(a1442, a1443)?
    }), f63(v0.clone(), v1, {
        let a1444 = v2;
        let a1445 = 2u64;
        nat_add(a1444, a1445)?
    }), f63(v0, v1, {
        let a1446 = v2;
        let a1447 = 3u64;
        nat_add(a1446, a1447)?
    })))
}

pub fn f57(v0: Fn10, p1451: u64, p1452: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let m1450 = (p1451, p1452);
    {
        let mut m1450 = m1450;
        loop {
            let step = {
                let (v1, v2) = m1450;
                let m1448 = v1;
                if m1448 == 0 {
                    Err::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>(v2)
                } else {
                    let v3 = m1448 - 1;
                    Ok::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>((v3, {
                        let c1449: Fn10 = v0.clone();
                        c1449.apply(v2)
                    }))
                }
            };
            match step {
                Ok(next) => { m1450 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f58(v0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let v1: u64 = {
        let (h1453, _) = v0;
        h1453
    };
    let v2: u32 = {
        let (h1454, _) = {
            let (_, h1455) = v0;
            h1455
        };
        h1454
    };
    let v3: u32 = {
        let (_, h1456) = {
            let (_, h1457) = v0;
            h1457
        };
        h1456
    };
    ({
        let a1458 = v1;
        let a1459 = 2u64;
        let a1460 = 0u64;
        nat_quot(a1458, a1459, a1460)
    }, (f64(v2, 1u32), {
        let a1464 = {
            let a1461 = v1;
            let a1462 = 2u64;
            let a1463 = 0u64;
            nat_rem(a1461, a1462, a1463)
        };
        let a1465 = 1u64;
        if nat_eq(a1464, a1465) {
            let a1466 = v3;
            let a1467 = v2;
            fixed_u32::bit_or(a1466, a1467)
        } else {
            v3
        }
    }))
}

pub fn f59(v0: Adt9) -> Adt4 {
    f65(Fn11::F66, Adt4::C0(0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32), match v0 {
        Adt9::C0(_, _, _, _, _, _, h1468) => {
            h1468
        }
    })
}

pub fn f60(v0: Adt1, v1: Adt4, v2: u32, v3: u32, v4: u32, v5: u32) -> R<Adt9> {
    let v6: (u64, Adt12) = f67(Fn12::F68(v0.clone(), v1.clone()), 800u64, (0u64, Adt12::C0(f69(v0.clone(), v2, v3, v4, v5), Adt4::C0(0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32), List::<Adt10>::nil())))?;
    Ok(Adt9::C0(v0, v1, v2, v3, v4, v5, {
        let m1471 = {
            let (_, h1470) = v6;
            h1470
        };
        match m1471 {
            Adt12::C0(_, _, h1469) => {
                h1469
            }
        }
    }))
}

pub fn f61(v0: Adt1, v1: u32, v2: Adt8) -> R<Option<Adt8>> {
    let a1476 = {
        let a1473 = match v2.clone() {
            Adt8::C0(_, h1472, _, _) => {
                h1472
            }
        };
        let a1474 = 2u64;
        let a1475 = 0u64;
        nat_rem(a1473, a1474, a1475)
    };
    let a1477 = 0u64;
    if nat_eq(a1476, a1477) {
        let m1479 = match v2.clone() {
            Adt8::C0(_, _, h1478, _) => {
                h1478
            }
        };
        match m1479.uncons() {
            None => {
                Ok(None::<Adt8>)
            }
            Some((v3, v4)) => {
                let v5: Adt3 = f29(f30(v3.clone(), match v2.clone() {
                    Adt8::C0(h1480, _, _, _) => {
                        h1480
                    }
                }, v0, v1)?)?;
                Ok(Some(Adt8::C0(Adt5::C0(f31(match v5.clone() {
                    Adt3::C0(_, _, _, _, _, _, _, _, _, _, _, h1481) => {
                        h1481
                    }
                }), match v3.clone() {
                    Adt5::C0(_, h1482, _) => {
                        h1482
                    }
                }, {
                    let a1487 = match v3.clone() {
                        Adt5::C0(_, _, h1483) => {
                            h1483
                        }
                    };
                    let a1488 = {
                        let m1486 = match v2.clone() {
                            Adt8::C0(h1485, _, _, _) => {
                                h1485
                            }
                        };
                        match m1486 {
                            Adt5::C0(_, _, h1484) => {
                                h1484
                            }
                        }
                    };
                    nat_add(a1487, a1488)?
                }), {
                    let a1490 = match v2.clone() {
                        Adt8::C0(_, h1489, _, _) => {
                            h1489
                        }
                    };
                    let a1491 = 2u64;
                    let a1492 = 0u64;
                    nat_quot(a1490, a1491, a1492)
                }, v4, List::cons(Adt2::C0(v5), List::cons(Adt2::C2(v3, {
                    let a1494 = match v2.clone() {
                        Adt8::C0(_, _, h1493, _) => {
                            h1493
                        }
                    };
                    length_list(a1494)
                }), match v2 {
                    Adt8::C0(_, _, _, h1495) => {
                        h1495
                    }
                })))))
            }
        }
    } else {
        Ok(None::<Adt8>)
    }
}

pub fn f62(v0: u32, v1: u32, v2: u32, v3: u32) -> u32 {
    let a1500 = v0;
    let a1501 = {
        let a1498 = f64(v1, 8u32);
        let a1499 = {
            let a1496 = f64(v2, 16u32);
            let a1497 = f64(v3, 24u32);
            fixed_u32::bit_or(a1496, a1497)
        };
        fixed_u32::bit_or(a1498, a1499)
    };
    fixed_u32::bit_or(a1500, a1501)
}

pub fn f63(v0: Bytes, v1: u64, v2: u64) -> u32 {
    let a1502 = v2;
    let a1503 = v1;
    if nat_lt(a1502, a1503) {
        let m1506 = {
            let a1504 = v0;
            let a1505 = v2;
            index_bytes(a1504, a1505)
        };
        match m1506 {
            None => {
                0u32
            }
            Some(v3) => {
                let m1508 = {
                    let a1507 = v3;
                    fixed_u32::convert(i128::from(a1507))
                };
                match m1508 {
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

pub fn f64(v0: u32, v1: u32) -> u32 {
    let m1511 = {
        let a1509 = v0;
        let a1510 = v1;
        fixed_u32::shift_left(a1509, a1510)
    };
    match m1511 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f65(v0: Fn11, p1515: Adt4, p1516: List<Adt10>) -> Adt4 {
    let m1514 = (p1515, p1516);
    {
        let mut m1514 = m1514;
        loop {
            let step = {
                let (v1, v2) = m1514;
                let m1512 = v2;
                match m1512.uncons() {
                    None => {
                        Err::<(Adt4, List<Adt10>), Adt4>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt4, List<Adt10>), Adt4>(({
                            let c1513: Fn11 = v0.clone();
                            c1513.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m1514 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f66(v0: Adt4, v1: Adt10) -> Adt4 {
    let a1518 = match v1.clone() {
        Adt10::C0(h1517, _, _, _, _, _, _, _, _) => {
            h1517
        }
    };
    let a1519 = 7u64;
    if nat_eq(a1518, a1519) {
        f70(v0, match v1.clone() {
            Adt10::C0(_, _, _, _, h1520, _, _, _, _) => {
                h1520
            }
        }, match v1 {
            Adt10::C0(_, _, _, _, _, _, _, _, h1521) => {
                h1521
            }
        })
    } else {
        v0
    }
}

pub fn f67(v0: Fn12, p1525: u64, p1526: (u64, Adt12)) -> R<(u64, Adt12)> {
    let m1524 = (p1525, p1526);
    {
        let mut m1524 = m1524;
        loop {
            let step = {
                let (v1, v2) = m1524;
                let m1522 = v1;
                if m1522 == 0 {
                    Err::<(u64, (u64, Adt12)), R<(u64, Adt12)>>(Ok(v2))
                } else {
                    let v3 = m1522 - 1;
                    Ok::<(u64, (u64, Adt12)), R<(u64, Adt12)>>((v3, {
                        let c1523: Fn12 = v0.clone();
                        c1523.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m1524 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f68(v0: Adt1, v1: Adt4, v2: (u64, Adt12)) -> R<(u64, Adt12)> {
    Ok(({
        let a1528 = {
            let (h1527, _) = v2.clone();
            h1527
        };
        let a1529 = 1u64;
        nat_add(a1528, a1529)?
    }, f71(v0, v1, {
        let (_, h1530) = v2;
        h1530
    }, {
        let (h1531, _) = v2;
        h1531
    })?))
}

pub fn f69(v0: Adt1, v1: u32, v2: u32, v3: u32, v4: u32) -> Adt4 {
    Adt4::C0(match v0.clone() {
        Adt1::C0(h1532, _, _, _, _, _, _, _) => {
            h1532
        }
    }, match v0.clone() {
        Adt1::C0(_, h1533, _, _, _, _, _, _) => {
            h1533
        }
    }, match v0.clone() {
        Adt1::C0(_, _, h1534, _, _, _, _, _) => {
            h1534
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, h1535, _, _, _, _) => {
            h1535
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, h1536, _, _, _) => {
            h1536
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, _, h1537, _, _) => {
            h1537
        }
    }, match v0.clone() {
        Adt1::C0(_, _, _, _, _, _, h1538, _) => {
            h1538
        }
    }, match v0 {
        Adt1::C0(_, _, _, _, _, _, _, h1539) => {
            h1539
        }
    }, 1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, v1, v2, v3, v4)
}

pub fn f70(v0: Adt4, v1: u64, v2: u32) -> Adt4 {
    let a1540 = v1;
    let a1541 = 8u64;
    if nat_lt(a1540, a1541) {
        let a1542 = v1;
        let a1543 = 4u64;
        if nat_lt(a1542, a1543) {
            let a1544 = v1;
            let a1545 = 2u64;
            if nat_lt(a1544, a1545) {
                let a1546 = v1;
                let a1547 = 1u64;
                if nat_lt(a1546, a1547) {
                    Adt4::C0(v2, match v0.clone() {
                        Adt4::C0(_, h1548, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1548
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1549, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1549
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1550, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1550
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1551, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1551
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1552, _, _, _, _, _, _, _, _, _, _) => {
                            h1552
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1553, _, _, _, _, _, _, _, _, _) => {
                            h1553
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1554, _, _, _, _, _, _, _, _) => {
                            h1554
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1555, _, _, _, _, _, _, _) => {
                            h1555
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1556, _, _, _, _, _, _) => {
                            h1556
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1557, _, _, _, _, _) => {
                            h1557
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1558, _, _, _, _) => {
                            h1558
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1559, _, _, _) => {
                            h1559
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1560, _, _) => {
                            h1560
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1561, _) => {
                            h1561
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1562) => {
                            h1562
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1563, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1563
                        }
                    }, v2, match v0.clone() {
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
                }
            } else {
                let a1578 = v1;
                let a1579 = 3u64;
                if nat_lt(a1578, a1579) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1580, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1580
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1581, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1581
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, h1582, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1582
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1583, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1583
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1584, _, _, _, _, _, _, _, _, _, _) => {
                            h1584
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1585, _, _, _, _, _, _, _, _, _) => {
                            h1585
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1586, _, _, _, _, _, _, _, _) => {
                            h1586
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1587, _, _, _, _, _, _, _) => {
                            h1587
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1588, _, _, _, _, _, _) => {
                            h1588
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1589, _, _, _, _, _) => {
                            h1589
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1590, _, _, _, _) => {
                            h1590
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1591, _, _, _) => {
                            h1591
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1592, _, _) => {
                            h1592
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1593, _) => {
                            h1593
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1594) => {
                            h1594
                        }
                    })
                } else {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1595, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1595
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1596, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1596
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1597, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1597
                        }
                    }, v2, match v0.clone() {
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
                }
            }
        } else {
            let a1610 = v1;
            let a1611 = 6u64;
            if nat_lt(a1610, a1611) {
                let a1612 = v1;
                let a1613 = 5u64;
                if nat_lt(a1612, a1613) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1614, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1614
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1615, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1615
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1616, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1616
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1617, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1617
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1618, _, _, _, _, _, _, _, _, _, _) => {
                            h1618
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1619, _, _, _, _, _, _, _, _, _) => {
                            h1619
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1620, _, _, _, _, _, _, _, _) => {
                            h1620
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1621, _, _, _, _, _, _, _) => {
                            h1621
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1622, _, _, _, _, _, _) => {
                            h1622
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1623, _, _, _, _, _) => {
                            h1623
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1624, _, _, _, _) => {
                            h1624
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1625, _, _, _) => {
                            h1625
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1626, _, _) => {
                            h1626
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1627, _) => {
                            h1627
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1628) => {
                            h1628
                        }
                    })
                } else {
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
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1633, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1633
                        }
                    }, v2, match v0.clone() {
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
                }
            } else {
                let a1644 = v1;
                let a1645 = 7u64;
                if nat_lt(a1644, a1645) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1646, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1646
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1647, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1647
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1648, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1648
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1649, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1649
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1650, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1650
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1651, _, _, _, _, _, _, _, _, _, _) => {
                            h1651
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1652, _, _, _, _, _, _, _, _) => {
                            h1652
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1653, _, _, _, _, _, _, _) => {
                            h1653
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1654, _, _, _, _, _, _) => {
                            h1654
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1655, _, _, _, _, _) => {
                            h1655
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1656, _, _, _, _) => {
                            h1656
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1657, _, _, _) => {
                            h1657
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1658, _, _) => {
                            h1658
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1659, _) => {
                            h1659
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1660) => {
                            h1660
                        }
                    })
                } else {
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
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1667, _, _, _, _, _, _, _, _, _) => {
                            h1667
                        }
                    }, v2, match v0.clone() {
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
                }
            }
        }
    } else {
        let a1676 = v1;
        let a1677 = 12u64;
        if nat_lt(a1676, a1677) {
            let a1678 = v1;
            let a1679 = 10u64;
            if nat_lt(a1678, a1679) {
                let a1680 = v1;
                let a1681 = 9u64;
                if nat_lt(a1680, a1681) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1682, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1682
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1683, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1683
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1684, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1684
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1685, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1685
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1686, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1686
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1687, _, _, _, _, _, _, _, _, _, _) => {
                            h1687
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1688, _, _, _, _, _, _, _, _, _) => {
                            h1688
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1689, _, _, _, _, _, _, _, _) => {
                            h1689
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1690, _, _, _, _, _, _) => {
                            h1690
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1691, _, _, _, _, _) => {
                            h1691
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1692, _, _, _, _) => {
                            h1692
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1693, _, _, _) => {
                            h1693
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1694, _, _) => {
                            h1694
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1695, _) => {
                            h1695
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1696) => {
                            h1696
                        }
                    })
                } else {
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
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1705, _, _, _, _, _, _, _) => {
                            h1705
                        }
                    }, v2, match v0.clone() {
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
                }
            } else {
                let a1712 = v1;
                let a1713 = 11u64;
                if nat_lt(a1712, a1713) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1714, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1714
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1715, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1715
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1716, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1716
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1717, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1717
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1718, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1718
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1719, _, _, _, _, _, _, _, _, _, _) => {
                            h1719
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1720, _, _, _, _, _, _, _, _, _) => {
                            h1720
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1721, _, _, _, _, _, _, _, _) => {
                            h1721
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1722, _, _, _, _, _, _, _) => {
                            h1722
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1723, _, _, _, _, _, _) => {
                            h1723
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1724, _, _, _, _) => {
                            h1724
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1725, _, _, _) => {
                            h1725
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1726, _, _) => {
                            h1726
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1727, _) => {
                            h1727
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1728) => {
                            h1728
                        }
                    })
                } else {
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
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1739, _, _, _, _, _) => {
                            h1739
                        }
                    }, v2, match v0.clone() {
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
                }
            }
        } else {
            let a1744 = v1;
            let a1745 = 14u64;
            if nat_lt(a1744, a1745) {
                let a1746 = v1;
                let a1747 = 13u64;
                if nat_lt(a1746, a1747) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1748, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1748
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1749, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1749
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1750, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1750
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1751, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1751
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1752, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1752
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1753, _, _, _, _, _, _, _, _, _, _) => {
                            h1753
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1754, _, _, _, _, _, _, _, _, _) => {
                            h1754
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1755, _, _, _, _, _, _, _, _) => {
                            h1755
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1756, _, _, _, _, _, _, _) => {
                            h1756
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1757, _, _, _, _, _, _) => {
                            h1757
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1758, _, _, _, _, _) => {
                            h1758
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1759, _, _, _, _) => {
                            h1759
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1760, _, _) => {
                            h1760
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1761, _) => {
                            h1761
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1762) => {
                            h1762
                        }
                    })
                } else {
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
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1775, _, _, _) => {
                            h1775
                        }
                    }, v2, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1776, _) => {
                            h1776
                        }
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1777) => {
                            h1777
                        }
                    })
                }
            } else {
                let a1778 = v1;
                let a1779 = 15u64;
                if nat_lt(a1778, a1779) {
                    Adt4::C0(match v0.clone() {
                        Adt4::C0(h1780, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1780
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, h1781, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1781
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, h1782, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1782
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, h1783, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1783
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, h1784, _, _, _, _, _, _, _, _, _, _, _) => {
                            h1784
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, h1785, _, _, _, _, _, _, _, _, _, _) => {
                            h1785
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, h1786, _, _, _, _, _, _, _, _, _) => {
                            h1786
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, h1787, _, _, _, _, _, _, _, _) => {
                            h1787
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, h1788, _, _, _, _, _, _, _) => {
                            h1788
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h1789, _, _, _, _, _, _) => {
                            h1789
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h1790, _, _, _, _, _) => {
                            h1790
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h1791, _, _, _, _) => {
                            h1791
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1792, _, _, _) => {
                            h1792
                        }
                    }, match v0.clone() {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1793, _, _) => {
                            h1793
                        }
                    }, v2, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1794) => {
                            h1794
                        }
                    })
                } else {
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
                    }, match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1809, _) => {
                            h1809
                        }
                    }, v2)
                }
            }
        }
    }
}

pub fn f71(v0: Adt1, v1: Adt4, v2: Adt12, v3: u64) -> R<Adt12> {
    let v4: u64 = {
        let a1810 = v3;
        let a1811 = 784u64;
        if nat_lt(a1810, a1811) {
            let a1812 = v3;
            let a1813 = 112u64;
            let a1814 = 0u64;
            nat_quot(a1812, a1813, a1814)
        } else {
            7u64
        }
    };
    let v5: Adt13 = f72({
        let a1815 = v3;
        let a1816 = 784u64;
        if nat_lt(a1815, a1816) {
            let a1817 = v3;
            let a1818 = 112u64;
            let a1819 = 0u64;
            nat_rem(a1817, a1818, a1819)
        } else {
            let a1820 = v3;
            let a1821 = 672u64;
            nat_sub(a1820, a1821)
        }
    });
    let v6: Adt4 = match v2.clone() {
        Adt12::C0(h1822, _, _) => {
            h1822
        }
    };
    let v7: u32 = f73(v6.clone(), match v5.clone() {
        Adt13::C0(_, _, _, h1823, _) => {
            h1823
        }
    });
    let v8: u32 = {
        let m1825 = match v5.clone() {
            Adt13::C0(_, _, h1824, _, _) => {
                h1824
            }
        };
        match m1825 {
            Adt11::C0 => {
                f73(v6.clone(), match v5.clone() {
                    Adt13::C0(_, _, _, _, h1826) => {
                        h1826
                    }
                })
            }
            Adt11::C1 => {
                f73(v1, f74(v4, match v5.clone() {
                    Adt13::C0(_, _, _, _, h1827) => {
                        h1827
                    }
                })?)
            }
            Adt11::C2 => {
                f73(v6.clone(), match v5.clone() {
                    Adt13::C0(_, _, _, _, h1828) => {
                        h1828
                    }
                })
            }
            Adt11::C3 => {
                f75(match v5.clone() {
                    Adt13::C0(_, _, _, _, h1829) => {
                        h1829
                    }
                })
            }
            Adt11::C4 => {
                f76(v0, match v5.clone() {
                    Adt13::C0(_, _, _, _, h1830) => {
                        h1830
                    }
                })
            }
        }
    };
    let v9: u32 = f77(match v5.clone() {
        Adt13::C0(_, _, h1831, _, _) => {
            h1831
        }
    }, v7, v8);
    Ok(Adt12::C0({
        let a1832 = v4;
        let a1833 = 7u64;
        if nat_eq(a1832, a1833) {
            v6
        } else {
            f70(v6, match v5.clone() {
                Adt13::C0(_, _, _, h1834, _) => {
                    h1834
                }
            }, v9)
        }
    }, {
        let a1835 = v4;
        let a1836 = 7u64;
        if nat_eq(a1835, a1836) {
            f70(match v2.clone() {
                Adt12::C0(_, h1837, _) => {
                    h1837
                }
            }, match v5.clone() {
                Adt13::C0(_, _, _, h1838, _) => {
                    h1838
                }
            }, v9)
        } else {
            match v2.clone() {
                Adt12::C0(_, h1839, _) => {
                    h1839
                }
            }
        }
    }, List::cons(Adt10::C0(v4, match v5.clone() {
        Adt13::C0(h1840, _, _, _, _) => {
            h1840
        }
    }, match v5.clone() {
        Adt13::C0(_, h1841, _, _, _) => {
            h1841
        }
    }, match v5.clone() {
        Adt13::C0(_, _, h1842, _, _) => {
            h1842
        }
    }, match v5.clone() {
        Adt13::C0(_, _, _, h1843, _) => {
            h1843
        }
    }, match v5 {
        Adt13::C0(_, _, _, _, h1844) => {
            h1844
        }
    }, v7, v8, v9), match v2 {
        Adt12::C0(_, _, h1845) => {
            h1845
        }
    })))
}

pub fn f72(v0: u64) -> Adt13 {
    let a1846 = v0;
    let a1847 = 64u64;
    if nat_lt(a1846, a1847) {
        let a1848 = v0;
        let a1849 = 32u64;
        if nat_lt(a1848, a1849) {
            let a1850 = v0;
            let a1851 = 16u64;
            if nat_lt(a1850, a1851) {
                let a1852 = v0;
                let a1853 = 8u64;
                if nat_lt(a1852, a1853) {
                    let a1854 = v0;
                    let a1855 = 4u64;
                    if nat_lt(a1854, a1855) {
                        let a1856 = v0;
                        let a1857 = 2u64;
                        if nat_lt(a1856, a1857) {
                            let a1858 = v0;
                            let a1859 = 1u64;
                            if nat_lt(a1858, a1859) {
                                Adt13::C0(0u64, 0u64, Adt11::C0, 0u64, 4u64)
                            } else {
                                Adt13::C0(0u64, 1u64, Adt11::C1, 0u64, 0u64)
                            }
                        } else {
                            let a1860 = v0;
                            let a1861 = 3u64;
                            if nat_lt(a1860, a1861) {
                                Adt13::C0(0u64, 2u64, Adt11::C2, 12u64, 0u64)
                            } else {
                                Adt13::C0(0u64, 3u64, Adt11::C3, 12u64, 16u64)
                            }
                        }
                    } else {
                        let a1862 = v0;
                        let a1863 = 6u64;
                        if nat_lt(a1862, a1863) {
                            let a1864 = v0;
                            let a1865 = 5u64;
                            if nat_lt(a1864, a1865) {
                                Adt13::C0(0u64, 4u64, Adt11::C0, 8u64, 12u64)
                            } else {
                                Adt13::C0(0u64, 5u64, Adt11::C2, 4u64, 8u64)
                            }
                        } else {
                            let a1866 = v0;
                            let a1867 = 7u64;
                            if nat_lt(a1866, a1867) {
                                Adt13::C0(0u64, 6u64, Adt11::C3, 4u64, 12u64)
                            } else {
                                Adt13::C0(0u64, 7u64, Adt11::C0, 0u64, 4u64)
                            }
                        }
                    }
                } else {
                    let a1868 = v0;
                    let a1869 = 12u64;
                    if nat_lt(a1868, a1869) {
                        let a1870 = v0;
                        let a1871 = 10u64;
                        if nat_lt(a1870, a1871) {
                            let a1872 = v0;
                            let a1873 = 9u64;
                            if nat_lt(a1872, a1873) {
                                Adt13::C0(0u64, 8u64, Adt11::C1, 0u64, 1u64)
                            } else {
                                Adt13::C0(0u64, 9u64, Adt11::C2, 12u64, 0u64)
                            }
                        } else {
                            let a1874 = v0;
                            let a1875 = 11u64;
                            if nat_lt(a1874, a1875) {
                                Adt13::C0(0u64, 10u64, Adt11::C3, 12u64, 8u64)
                            } else {
                                Adt13::C0(0u64, 11u64, Adt11::C0, 8u64, 12u64)
                            }
                        }
                    } else {
                        let a1876 = v0;
                        let a1877 = 14u64;
                        if nat_lt(a1876, a1877) {
                            let a1878 = v0;
                            let a1879 = 13u64;
                            if nat_lt(a1878, a1879) {
                                Adt13::C0(0u64, 12u64, Adt11::C2, 4u64, 8u64)
                            } else {
                                Adt13::C0(0u64, 13u64, Adt11::C3, 4u64, 7u64)
                            }
                        } else {
                            let a1880 = v0;
                            let a1881 = 15u64;
                            if nat_lt(a1880, a1881) {
                                Adt13::C0(1u64, 0u64, Adt11::C0, 1u64, 5u64)
                            } else {
                                Adt13::C0(1u64, 1u64, Adt11::C1, 1u64, 2u64)
                            }
                        }
                    }
                }
            } else {
                let a1882 = v0;
                let a1883 = 24u64;
                if nat_lt(a1882, a1883) {
                    let a1884 = v0;
                    let a1885 = 20u64;
                    if nat_lt(a1884, a1885) {
                        let a1886 = v0;
                        let a1887 = 18u64;
                        if nat_lt(a1886, a1887) {
                            let a1888 = v0;
                            let a1889 = 17u64;
                            if nat_lt(a1888, a1889) {
                                Adt13::C0(1u64, 2u64, Adt11::C2, 13u64, 1u64)
                            } else {
                                Adt13::C0(1u64, 3u64, Adt11::C3, 13u64, 16u64)
                            }
                        } else {
                            let a1890 = v0;
                            let a1891 = 19u64;
                            if nat_lt(a1890, a1891) {
                                Adt13::C0(1u64, 4u64, Adt11::C0, 9u64, 13u64)
                            } else {
                                Adt13::C0(1u64, 5u64, Adt11::C2, 5u64, 9u64)
                            }
                        }
                    } else {
                        let a1892 = v0;
                        let a1893 = 22u64;
                        if nat_lt(a1892, a1893) {
                            let a1894 = v0;
                            let a1895 = 21u64;
                            if nat_lt(a1894, a1895) {
                                Adt13::C0(1u64, 6u64, Adt11::C3, 5u64, 12u64)
                            } else {
                                Adt13::C0(1u64, 7u64, Adt11::C0, 1u64, 5u64)
                            }
                        } else {
                            let a1896 = v0;
                            let a1897 = 23u64;
                            if nat_lt(a1896, a1897) {
                                Adt13::C0(1u64, 8u64, Adt11::C1, 1u64, 3u64)
                            } else {
                                Adt13::C0(1u64, 9u64, Adt11::C2, 13u64, 1u64)
                            }
                        }
                    }
                } else {
                    let a1898 = v0;
                    let a1899 = 28u64;
                    if nat_lt(a1898, a1899) {
                        let a1900 = v0;
                        let a1901 = 26u64;
                        if nat_lt(a1900, a1901) {
                            let a1902 = v0;
                            let a1903 = 25u64;
                            if nat_lt(a1902, a1903) {
                                Adt13::C0(1u64, 10u64, Adt11::C3, 13u64, 8u64)
                            } else {
                                Adt13::C0(1u64, 11u64, Adt11::C0, 9u64, 13u64)
                            }
                        } else {
                            let a1904 = v0;
                            let a1905 = 27u64;
                            if nat_lt(a1904, a1905) {
                                Adt13::C0(1u64, 12u64, Adt11::C2, 5u64, 9u64)
                            } else {
                                Adt13::C0(1u64, 13u64, Adt11::C3, 5u64, 7u64)
                            }
                        }
                    } else {
                        let a1906 = v0;
                        let a1907 = 30u64;
                        if nat_lt(a1906, a1907) {
                            let a1908 = v0;
                            let a1909 = 29u64;
                            if nat_lt(a1908, a1909) {
                                Adt13::C0(2u64, 0u64, Adt11::C0, 2u64, 6u64)
                            } else {
                                Adt13::C0(2u64, 1u64, Adt11::C1, 2u64, 4u64)
                            }
                        } else {
                            let a1910 = v0;
                            let a1911 = 31u64;
                            if nat_lt(a1910, a1911) {
                                Adt13::C0(2u64, 2u64, Adt11::C2, 14u64, 2u64)
                            } else {
                                Adt13::C0(2u64, 3u64, Adt11::C3, 14u64, 16u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a1912 = v0;
            let a1913 = 48u64;
            if nat_lt(a1912, a1913) {
                let a1914 = v0;
                let a1915 = 40u64;
                if nat_lt(a1914, a1915) {
                    let a1916 = v0;
                    let a1917 = 36u64;
                    if nat_lt(a1916, a1917) {
                        let a1918 = v0;
                        let a1919 = 34u64;
                        if nat_lt(a1918, a1919) {
                            let a1920 = v0;
                            let a1921 = 33u64;
                            if nat_lt(a1920, a1921) {
                                Adt13::C0(2u64, 4u64, Adt11::C0, 10u64, 14u64)
                            } else {
                                Adt13::C0(2u64, 5u64, Adt11::C2, 6u64, 10u64)
                            }
                        } else {
                            let a1922 = v0;
                            let a1923 = 35u64;
                            if nat_lt(a1922, a1923) {
                                Adt13::C0(2u64, 6u64, Adt11::C3, 6u64, 12u64)
                            } else {
                                Adt13::C0(2u64, 7u64, Adt11::C0, 2u64, 6u64)
                            }
                        }
                    } else {
                        let a1924 = v0;
                        let a1925 = 38u64;
                        if nat_lt(a1924, a1925) {
                            let a1926 = v0;
                            let a1927 = 37u64;
                            if nat_lt(a1926, a1927) {
                                Adt13::C0(2u64, 8u64, Adt11::C1, 2u64, 5u64)
                            } else {
                                Adt13::C0(2u64, 9u64, Adt11::C2, 14u64, 2u64)
                            }
                        } else {
                            let a1928 = v0;
                            let a1929 = 39u64;
                            if nat_lt(a1928, a1929) {
                                Adt13::C0(2u64, 10u64, Adt11::C3, 14u64, 8u64)
                            } else {
                                Adt13::C0(2u64, 11u64, Adt11::C0, 10u64, 14u64)
                            }
                        }
                    }
                } else {
                    let a1930 = v0;
                    let a1931 = 44u64;
                    if nat_lt(a1930, a1931) {
                        let a1932 = v0;
                        let a1933 = 42u64;
                        if nat_lt(a1932, a1933) {
                            let a1934 = v0;
                            let a1935 = 41u64;
                            if nat_lt(a1934, a1935) {
                                Adt13::C0(2u64, 12u64, Adt11::C2, 6u64, 10u64)
                            } else {
                                Adt13::C0(2u64, 13u64, Adt11::C3, 6u64, 7u64)
                            }
                        } else {
                            let a1936 = v0;
                            let a1937 = 43u64;
                            if nat_lt(a1936, a1937) {
                                Adt13::C0(3u64, 0u64, Adt11::C0, 3u64, 7u64)
                            } else {
                                Adt13::C0(3u64, 1u64, Adt11::C1, 3u64, 6u64)
                            }
                        }
                    } else {
                        let a1938 = v0;
                        let a1939 = 46u64;
                        if nat_lt(a1938, a1939) {
                            let a1940 = v0;
                            let a1941 = 45u64;
                            if nat_lt(a1940, a1941) {
                                Adt13::C0(3u64, 2u64, Adt11::C2, 15u64, 3u64)
                            } else {
                                Adt13::C0(3u64, 3u64, Adt11::C3, 15u64, 16u64)
                            }
                        } else {
                            let a1942 = v0;
                            let a1943 = 47u64;
                            if nat_lt(a1942, a1943) {
                                Adt13::C0(3u64, 4u64, Adt11::C0, 11u64, 15u64)
                            } else {
                                Adt13::C0(3u64, 5u64, Adt11::C2, 7u64, 11u64)
                            }
                        }
                    }
                }
            } else {
                let a1944 = v0;
                let a1945 = 56u64;
                if nat_lt(a1944, a1945) {
                    let a1946 = v0;
                    let a1947 = 52u64;
                    if nat_lt(a1946, a1947) {
                        let a1948 = v0;
                        let a1949 = 50u64;
                        if nat_lt(a1948, a1949) {
                            let a1950 = v0;
                            let a1951 = 49u64;
                            if nat_lt(a1950, a1951) {
                                Adt13::C0(3u64, 6u64, Adt11::C3, 7u64, 12u64)
                            } else {
                                Adt13::C0(3u64, 7u64, Adt11::C0, 3u64, 7u64)
                            }
                        } else {
                            let a1952 = v0;
                            let a1953 = 51u64;
                            if nat_lt(a1952, a1953) {
                                Adt13::C0(3u64, 8u64, Adt11::C1, 3u64, 7u64)
                            } else {
                                Adt13::C0(3u64, 9u64, Adt11::C2, 15u64, 3u64)
                            }
                        }
                    } else {
                        let a1954 = v0;
                        let a1955 = 54u64;
                        if nat_lt(a1954, a1955) {
                            let a1956 = v0;
                            let a1957 = 53u64;
                            if nat_lt(a1956, a1957) {
                                Adt13::C0(3u64, 10u64, Adt11::C3, 15u64, 8u64)
                            } else {
                                Adt13::C0(3u64, 11u64, Adt11::C0, 11u64, 15u64)
                            }
                        } else {
                            let a1958 = v0;
                            let a1959 = 55u64;
                            if nat_lt(a1958, a1959) {
                                Adt13::C0(3u64, 12u64, Adt11::C2, 7u64, 11u64)
                            } else {
                                Adt13::C0(3u64, 13u64, Adt11::C3, 7u64, 7u64)
                            }
                        }
                    }
                } else {
                    let a1960 = v0;
                    let a1961 = 60u64;
                    if nat_lt(a1960, a1961) {
                        let a1962 = v0;
                        let a1963 = 58u64;
                        if nat_lt(a1962, a1963) {
                            let a1964 = v0;
                            let a1965 = 57u64;
                            if nat_lt(a1964, a1965) {
                                Adt13::C0(4u64, 0u64, Adt11::C0, 0u64, 5u64)
                            } else {
                                Adt13::C0(4u64, 1u64, Adt11::C1, 0u64, 8u64)
                            }
                        } else {
                            let a1966 = v0;
                            let a1967 = 59u64;
                            if nat_lt(a1966, a1967) {
                                Adt13::C0(4u64, 2u64, Adt11::C2, 15u64, 0u64)
                            } else {
                                Adt13::C0(4u64, 3u64, Adt11::C3, 15u64, 16u64)
                            }
                        }
                    } else {
                        let a1968 = v0;
                        let a1969 = 62u64;
                        if nat_lt(a1968, a1969) {
                            let a1970 = v0;
                            let a1971 = 61u64;
                            if nat_lt(a1970, a1971) {
                                Adt13::C0(4u64, 4u64, Adt11::C0, 10u64, 15u64)
                            } else {
                                Adt13::C0(4u64, 5u64, Adt11::C2, 5u64, 10u64)
                            }
                        } else {
                            let a1972 = v0;
                            let a1973 = 63u64;
                            if nat_lt(a1972, a1973) {
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
        let a1974 = v0;
        let a1975 = 96u64;
        if nat_lt(a1974, a1975) {
            let a1976 = v0;
            let a1977 = 80u64;
            if nat_lt(a1976, a1977) {
                let a1978 = v0;
                let a1979 = 72u64;
                if nat_lt(a1978, a1979) {
                    let a1980 = v0;
                    let a1981 = 68u64;
                    if nat_lt(a1980, a1981) {
                        let a1982 = v0;
                        let a1983 = 66u64;
                        if nat_lt(a1982, a1983) {
                            let a1984 = v0;
                            let a1985 = 65u64;
                            if nat_lt(a1984, a1985) {
                                Adt13::C0(4u64, 8u64, Adt11::C1, 0u64, 9u64)
                            } else {
                                Adt13::C0(4u64, 9u64, Adt11::C2, 15u64, 0u64)
                            }
                        } else {
                            let a1986 = v0;
                            let a1987 = 67u64;
                            if nat_lt(a1986, a1987) {
                                Adt13::C0(4u64, 10u64, Adt11::C3, 15u64, 8u64)
                            } else {
                                Adt13::C0(4u64, 11u64, Adt11::C0, 10u64, 15u64)
                            }
                        }
                    } else {
                        let a1988 = v0;
                        let a1989 = 70u64;
                        if nat_lt(a1988, a1989) {
                            let a1990 = v0;
                            let a1991 = 69u64;
                            if nat_lt(a1990, a1991) {
                                Adt13::C0(4u64, 12u64, Adt11::C2, 5u64, 10u64)
                            } else {
                                Adt13::C0(4u64, 13u64, Adt11::C3, 5u64, 7u64)
                            }
                        } else {
                            let a1992 = v0;
                            let a1993 = 71u64;
                            if nat_lt(a1992, a1993) {
                                Adt13::C0(5u64, 0u64, Adt11::C0, 1u64, 6u64)
                            } else {
                                Adt13::C0(5u64, 1u64, Adt11::C1, 1u64, 10u64)
                            }
                        }
                    }
                } else {
                    let a1994 = v0;
                    let a1995 = 76u64;
                    if nat_lt(a1994, a1995) {
                        let a1996 = v0;
                        let a1997 = 74u64;
                        if nat_lt(a1996, a1997) {
                            let a1998 = v0;
                            let a1999 = 73u64;
                            if nat_lt(a1998, a1999) {
                                Adt13::C0(5u64, 2u64, Adt11::C2, 12u64, 1u64)
                            } else {
                                Adt13::C0(5u64, 3u64, Adt11::C3, 12u64, 16u64)
                            }
                        } else {
                            let a2000 = v0;
                            let a2001 = 75u64;
                            if nat_lt(a2000, a2001) {
                                Adt13::C0(5u64, 4u64, Adt11::C0, 11u64, 12u64)
                            } else {
                                Adt13::C0(5u64, 5u64, Adt11::C2, 6u64, 11u64)
                            }
                        }
                    } else {
                        let a2002 = v0;
                        let a2003 = 78u64;
                        if nat_lt(a2002, a2003) {
                            let a2004 = v0;
                            let a2005 = 77u64;
                            if nat_lt(a2004, a2005) {
                                Adt13::C0(5u64, 6u64, Adt11::C3, 6u64, 12u64)
                            } else {
                                Adt13::C0(5u64, 7u64, Adt11::C0, 1u64, 6u64)
                            }
                        } else {
                            let a2006 = v0;
                            let a2007 = 79u64;
                            if nat_lt(a2006, a2007) {
                                Adt13::C0(5u64, 8u64, Adt11::C1, 1u64, 11u64)
                            } else {
                                Adt13::C0(5u64, 9u64, Adt11::C2, 12u64, 1u64)
                            }
                        }
                    }
                }
            } else {
                let a2008 = v0;
                let a2009 = 88u64;
                if nat_lt(a2008, a2009) {
                    let a2010 = v0;
                    let a2011 = 84u64;
                    if nat_lt(a2010, a2011) {
                        let a2012 = v0;
                        let a2013 = 82u64;
                        if nat_lt(a2012, a2013) {
                            let a2014 = v0;
                            let a2015 = 81u64;
                            if nat_lt(a2014, a2015) {
                                Adt13::C0(5u64, 10u64, Adt11::C3, 12u64, 8u64)
                            } else {
                                Adt13::C0(5u64, 11u64, Adt11::C0, 11u64, 12u64)
                            }
                        } else {
                            let a2016 = v0;
                            let a2017 = 83u64;
                            if nat_lt(a2016, a2017) {
                                Adt13::C0(5u64, 12u64, Adt11::C2, 6u64, 11u64)
                            } else {
                                Adt13::C0(5u64, 13u64, Adt11::C3, 6u64, 7u64)
                            }
                        }
                    } else {
                        let a2018 = v0;
                        let a2019 = 86u64;
                        if nat_lt(a2018, a2019) {
                            let a2020 = v0;
                            let a2021 = 85u64;
                            if nat_lt(a2020, a2021) {
                                Adt13::C0(6u64, 0u64, Adt11::C0, 2u64, 7u64)
                            } else {
                                Adt13::C0(6u64, 1u64, Adt11::C1, 2u64, 12u64)
                            }
                        } else {
                            let a2022 = v0;
                            let a2023 = 87u64;
                            if nat_lt(a2022, a2023) {
                                Adt13::C0(6u64, 2u64, Adt11::C2, 13u64, 2u64)
                            } else {
                                Adt13::C0(6u64, 3u64, Adt11::C3, 13u64, 16u64)
                            }
                        }
                    }
                } else {
                    let a2024 = v0;
                    let a2025 = 92u64;
                    if nat_lt(a2024, a2025) {
                        let a2026 = v0;
                        let a2027 = 90u64;
                        if nat_lt(a2026, a2027) {
                            let a2028 = v0;
                            let a2029 = 89u64;
                            if nat_lt(a2028, a2029) {
                                Adt13::C0(6u64, 4u64, Adt11::C0, 8u64, 13u64)
                            } else {
                                Adt13::C0(6u64, 5u64, Adt11::C2, 7u64, 8u64)
                            }
                        } else {
                            let a2030 = v0;
                            let a2031 = 91u64;
                            if nat_lt(a2030, a2031) {
                                Adt13::C0(6u64, 6u64, Adt11::C3, 7u64, 12u64)
                            } else {
                                Adt13::C0(6u64, 7u64, Adt11::C0, 2u64, 7u64)
                            }
                        }
                    } else {
                        let a2032 = v0;
                        let a2033 = 94u64;
                        if nat_lt(a2032, a2033) {
                            let a2034 = v0;
                            let a2035 = 93u64;
                            if nat_lt(a2034, a2035) {
                                Adt13::C0(6u64, 8u64, Adt11::C1, 2u64, 13u64)
                            } else {
                                Adt13::C0(6u64, 9u64, Adt11::C2, 13u64, 2u64)
                            }
                        } else {
                            let a2036 = v0;
                            let a2037 = 95u64;
                            if nat_lt(a2036, a2037) {
                                Adt13::C0(6u64, 10u64, Adt11::C3, 13u64, 8u64)
                            } else {
                                Adt13::C0(6u64, 11u64, Adt11::C0, 8u64, 13u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a2038 = v0;
            let a2039 = 112u64;
            if nat_lt(a2038, a2039) {
                let a2040 = v0;
                let a2041 = 104u64;
                if nat_lt(a2040, a2041) {
                    let a2042 = v0;
                    let a2043 = 100u64;
                    if nat_lt(a2042, a2043) {
                        let a2044 = v0;
                        let a2045 = 98u64;
                        if nat_lt(a2044, a2045) {
                            let a2046 = v0;
                            let a2047 = 97u64;
                            if nat_lt(a2046, a2047) {
                                Adt13::C0(6u64, 12u64, Adt11::C2, 7u64, 8u64)
                            } else {
                                Adt13::C0(6u64, 13u64, Adt11::C3, 7u64, 7u64)
                            }
                        } else {
                            let a2048 = v0;
                            let a2049 = 99u64;
                            if nat_lt(a2048, a2049) {
                                Adt13::C0(7u64, 0u64, Adt11::C0, 3u64, 4u64)
                            } else {
                                Adt13::C0(7u64, 1u64, Adt11::C1, 3u64, 14u64)
                            }
                        }
                    } else {
                        let a2050 = v0;
                        let a2051 = 102u64;
                        if nat_lt(a2050, a2051) {
                            let a2052 = v0;
                            let a2053 = 101u64;
                            if nat_lt(a2052, a2053) {
                                Adt13::C0(7u64, 2u64, Adt11::C2, 14u64, 3u64)
                            } else {
                                Adt13::C0(7u64, 3u64, Adt11::C3, 14u64, 16u64)
                            }
                        } else {
                            let a2054 = v0;
                            let a2055 = 103u64;
                            if nat_lt(a2054, a2055) {
                                Adt13::C0(7u64, 4u64, Adt11::C0, 9u64, 14u64)
                            } else {
                                Adt13::C0(7u64, 5u64, Adt11::C2, 4u64, 9u64)
                            }
                        }
                    }
                } else {
                    let a2056 = v0;
                    let a2057 = 108u64;
                    if nat_lt(a2056, a2057) {
                        let a2058 = v0;
                        let a2059 = 106u64;
                        if nat_lt(a2058, a2059) {
                            let a2060 = v0;
                            let a2061 = 105u64;
                            if nat_lt(a2060, a2061) {
                                Adt13::C0(7u64, 6u64, Adt11::C3, 4u64, 12u64)
                            } else {
                                Adt13::C0(7u64, 7u64, Adt11::C0, 3u64, 4u64)
                            }
                        } else {
                            let a2062 = v0;
                            let a2063 = 107u64;
                            if nat_lt(a2062, a2063) {
                                Adt13::C0(7u64, 8u64, Adt11::C1, 3u64, 15u64)
                            } else {
                                Adt13::C0(7u64, 9u64, Adt11::C2, 14u64, 3u64)
                            }
                        }
                    } else {
                        let a2064 = v0;
                        let a2065 = 110u64;
                        if nat_lt(a2064, a2065) {
                            let a2066 = v0;
                            let a2067 = 109u64;
                            if nat_lt(a2066, a2067) {
                                Adt13::C0(7u64, 10u64, Adt11::C3, 14u64, 8u64)
                            } else {
                                Adt13::C0(7u64, 11u64, Adt11::C0, 9u64, 14u64)
                            }
                        } else {
                            let a2068 = v0;
                            let a2069 = 111u64;
                            if nat_lt(a2068, a2069) {
                                Adt13::C0(7u64, 12u64, Adt11::C2, 4u64, 9u64)
                            } else {
                                Adt13::C0(7u64, 13u64, Adt11::C3, 4u64, 7u64)
                            }
                        }
                    }
                }
            } else {
                let a2070 = v0;
                let a2071 = 120u64;
                if nat_lt(a2070, a2071) {
                    let a2072 = v0;
                    let a2073 = 116u64;
                    if nat_lt(a2072, a2073) {
                        let a2074 = v0;
                        let a2075 = 114u64;
                        if nat_lt(a2074, a2075) {
                            let a2076 = v0;
                            let a2077 = 113u64;
                            if nat_lt(a2076, a2077) {
                                Adt13::C0(0u64, 0u64, Adt11::C2, 0u64, 8u64)
                            } else {
                                Adt13::C0(0u64, 1u64, Adt11::C2, 1u64, 9u64)
                            }
                        } else {
                            let a2078 = v0;
                            let a2079 = 115u64;
                            if nat_lt(a2078, a2079) {
                                Adt13::C0(0u64, 2u64, Adt11::C2, 2u64, 10u64)
                            } else {
                                Adt13::C0(0u64, 3u64, Adt11::C2, 3u64, 11u64)
                            }
                        }
                    } else {
                        let a2080 = v0;
                        let a2081 = 118u64;
                        if nat_lt(a2080, a2081) {
                            let a2082 = v0;
                            let a2083 = 117u64;
                            if nat_lt(a2082, a2083) {
                                Adt13::C0(0u64, 4u64, Adt11::C2, 4u64, 12u64)
                            } else {
                                Adt13::C0(0u64, 5u64, Adt11::C2, 5u64, 13u64)
                            }
                        } else {
                            let a2084 = v0;
                            let a2085 = 119u64;
                            if nat_lt(a2084, a2085) {
                                Adt13::C0(0u64, 6u64, Adt11::C2, 6u64, 14u64)
                            } else {
                                Adt13::C0(0u64, 7u64, Adt11::C2, 7u64, 15u64)
                            }
                        }
                    }
                } else {
                    let a2086 = v0;
                    let a2087 = 124u64;
                    if nat_lt(a2086, a2087) {
                        let a2088 = v0;
                        let a2089 = 122u64;
                        if nat_lt(a2088, a2089) {
                            let a2090 = v0;
                            let a2091 = 121u64;
                            if nat_lt(a2090, a2091) {
                                Adt13::C0(0u64, 8u64, Adt11::C4, 8u64, 0u64)
                            } else {
                                Adt13::C0(0u64, 9u64, Adt11::C4, 9u64, 1u64)
                            }
                        } else {
                            let a2092 = v0;
                            let a2093 = 123u64;
                            if nat_lt(a2092, a2093) {
                                Adt13::C0(0u64, 10u64, Adt11::C4, 10u64, 2u64)
                            } else {
                                Adt13::C0(0u64, 11u64, Adt11::C4, 11u64, 3u64)
                            }
                        }
                    } else {
                        let a2094 = v0;
                        let a2095 = 126u64;
                        if nat_lt(a2094, a2095) {
                            let a2096 = v0;
                            let a2097 = 125u64;
                            if nat_lt(a2096, a2097) {
                                Adt13::C0(0u64, 12u64, Adt11::C4, 12u64, 4u64)
                            } else {
                                Adt13::C0(0u64, 13u64, Adt11::C4, 13u64, 5u64)
                            }
                        } else {
                            let a2098 = v0;
                            let a2099 = 127u64;
                            if nat_lt(a2098, a2099) {
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

pub fn f73(v0: Adt4, v1: u64) -> u32 {
    let a2100 = v1;
    let a2101 = 8u64;
    if nat_lt(a2100, a2101) {
        let a2102 = v1;
        let a2103 = 4u64;
        if nat_lt(a2102, a2103) {
            let a2104 = v1;
            let a2105 = 2u64;
            if nat_lt(a2104, a2105) {
                let a2106 = v1;
                let a2107 = 1u64;
                if nat_lt(a2106, a2107) {
                    match v0 {
                        Adt4::C0(h2108, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2108
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, h2109, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2109
                        }
                    }
                }
            } else {
                let a2110 = v1;
                let a2111 = 3u64;
                if nat_lt(a2110, a2111) {
                    match v0 {
                        Adt4::C0(_, _, h2112, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2112
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, h2113, _, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2113
                        }
                    }
                }
            }
        } else {
            let a2114 = v1;
            let a2115 = 6u64;
            if nat_lt(a2114, a2115) {
                let a2116 = v1;
                let a2117 = 5u64;
                if nat_lt(a2116, a2117) {
                    match v0 {
                        Adt4::C0(_, _, _, _, h2118, _, _, _, _, _, _, _, _, _, _, _) => {
                            h2118
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, h2119, _, _, _, _, _, _, _, _, _, _) => {
                            h2119
                        }
                    }
                }
            } else {
                let a2120 = v1;
                let a2121 = 7u64;
                if nat_lt(a2120, a2121) {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, h2122, _, _, _, _, _, _, _, _, _) => {
                            h2122
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, h2123, _, _, _, _, _, _, _, _) => {
                            h2123
                        }
                    }
                }
            }
        }
    } else {
        let a2124 = v1;
        let a2125 = 12u64;
        if nat_lt(a2124, a2125) {
            let a2126 = v1;
            let a2127 = 10u64;
            if nat_lt(a2126, a2127) {
                let a2128 = v1;
                let a2129 = 9u64;
                if nat_lt(a2128, a2129) {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, h2130, _, _, _, _, _, _, _) => {
                            h2130
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, h2131, _, _, _, _, _, _) => {
                            h2131
                        }
                    }
                }
            } else {
                let a2132 = v1;
                let a2133 = 11u64;
                if nat_lt(a2132, a2133) {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, h2134, _, _, _, _, _) => {
                            h2134
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, h2135, _, _, _, _) => {
                            h2135
                        }
                    }
                }
            }
        } else {
            let a2136 = v1;
            let a2137 = 14u64;
            if nat_lt(a2136, a2137) {
                let a2138 = v1;
                let a2139 = 13u64;
                if nat_lt(a2138, a2139) {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, h2140, _, _, _) => {
                            h2140
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h2141, _, _) => {
                            h2141
                        }
                    }
                }
            } else {
                let a2142 = v1;
                let a2143 = 15u64;
                if nat_lt(a2142, a2143) {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h2144, _) => {
                            h2144
                        }
                    }
                } else {
                    match v0 {
                        Adt4::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h2145) => {
                            h2145
                        }
                    }
                }
            }
        }
    }
}

pub fn f74(v0: u64, v1: u64) -> R<u64> {
    let v2: u64 = {
        let a2148 = {
            let a2146 = v0;
            let a2147 = 16u64;
            nat_mul(a2146, a2147)?
        };
        let a2149 = v1;
        nat_add(a2148, a2149)?
    };
    let a2150 = v2;
    let a2151 = 56u64;
    if nat_lt(a2150, a2151) {
        let a2152 = v2;
        let a2153 = 28u64;
        if nat_lt(a2152, a2153) {
            let a2154 = v2;
            let a2155 = 14u64;
            if nat_lt(a2154, a2155) {
                let a2156 = v2;
                let a2157 = 7u64;
                if nat_lt(a2156, a2157) {
                    let a2158 = v2;
                    let a2159 = 3u64;
                    if nat_lt(a2158, a2159) {
                        let a2160 = v2;
                        let a2161 = 1u64;
                        if nat_lt(a2160, a2161) {
                            Ok(0u64)
                        } else {
                            let a2162 = v2;
                            let a2163 = 2u64;
                            if nat_lt(a2162, a2163) {
                                Ok(1u64)
                            } else {
                                Ok(2u64)
                            }
                        }
                    } else {
                        let a2164 = v2;
                        let a2165 = 5u64;
                        if nat_lt(a2164, a2165) {
                            let a2166 = v2;
                            let a2167 = 4u64;
                            if nat_lt(a2166, a2167) {
                                Ok(3u64)
                            } else {
                                Ok(4u64)
                            }
                        } else {
                            let a2168 = v2;
                            let a2169 = 6u64;
                            if nat_lt(a2168, a2169) {
                                Ok(5u64)
                            } else {
                                Ok(6u64)
                            }
                        }
                    }
                } else {
                    let a2170 = v2;
                    let a2171 = 10u64;
                    if nat_lt(a2170, a2171) {
                        let a2172 = v2;
                        let a2173 = 8u64;
                        if nat_lt(a2172, a2173) {
                            Ok(7u64)
                        } else {
                            let a2174 = v2;
                            let a2175 = 9u64;
                            if nat_lt(a2174, a2175) {
                                Ok(8u64)
                            } else {
                                Ok(9u64)
                            }
                        }
                    } else {
                        let a2176 = v2;
                        let a2177 = 12u64;
                        if nat_lt(a2176, a2177) {
                            let a2178 = v2;
                            let a2179 = 11u64;
                            if nat_lt(a2178, a2179) {
                                Ok(10u64)
                            } else {
                                Ok(11u64)
                            }
                        } else {
                            let a2180 = v2;
                            let a2181 = 13u64;
                            if nat_lt(a2180, a2181) {
                                Ok(12u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    }
                }
            } else {
                let a2182 = v2;
                let a2183 = 21u64;
                if nat_lt(a2182, a2183) {
                    let a2184 = v2;
                    let a2185 = 17u64;
                    if nat_lt(a2184, a2185) {
                        let a2186 = v2;
                        let a2187 = 15u64;
                        if nat_lt(a2186, a2187) {
                            Ok(14u64)
                        } else {
                            let a2188 = v2;
                            let a2189 = 16u64;
                            if nat_lt(a2188, a2189) {
                                Ok(15u64)
                            } else {
                                Ok(2u64)
                            }
                        }
                    } else {
                        let a2190 = v2;
                        let a2191 = 19u64;
                        if nat_lt(a2190, a2191) {
                            let a2192 = v2;
                            let a2193 = 18u64;
                            if nat_lt(a2192, a2193) {
                                Ok(6u64)
                            } else {
                                Ok(3u64)
                            }
                        } else {
                            let a2194 = v2;
                            let a2195 = 20u64;
                            if nat_lt(a2194, a2195) {
                                Ok(10u64)
                            } else {
                                Ok(7u64)
                            }
                        }
                    }
                } else {
                    let a2196 = v2;
                    let a2197 = 24u64;
                    if nat_lt(a2196, a2197) {
                        let a2198 = v2;
                        let a2199 = 22u64;
                        if nat_lt(a2198, a2199) {
                            Ok(0u64)
                        } else {
                            let a2200 = v2;
                            let a2201 = 23u64;
                            if nat_lt(a2200, a2201) {
                                Ok(4u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    } else {
                        let a2202 = v2;
                        let a2203 = 26u64;
                        if nat_lt(a2202, a2203) {
                            let a2204 = v2;
                            let a2205 = 25u64;
                            if nat_lt(a2204, a2205) {
                                Ok(1u64)
                            } else {
                                Ok(11u64)
                            }
                        } else {
                            let a2206 = v2;
                            let a2207 = 27u64;
                            if nat_lt(a2206, a2207) {
                                Ok(12u64)
                            } else {
                                Ok(5u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a2208 = v2;
            let a2209 = 42u64;
            if nat_lt(a2208, a2209) {
                let a2210 = v2;
                let a2211 = 35u64;
                if nat_lt(a2210, a2211) {
                    let a2212 = v2;
                    let a2213 = 31u64;
                    if nat_lt(a2212, a2213) {
                        let a2214 = v2;
                        let a2215 = 29u64;
                        if nat_lt(a2214, a2215) {
                            Ok(9u64)
                        } else {
                            let a2216 = v2;
                            let a2217 = 30u64;
                            if nat_lt(a2216, a2217) {
                                Ok(14u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    } else {
                        let a2218 = v2;
                        let a2219 = 33u64;
                        if nat_lt(a2218, a2219) {
                            let a2220 = v2;
                            let a2221 = 32u64;
                            if nat_lt(a2220, a2221) {
                                Ok(8u64)
                            } else {
                                Ok(3u64)
                            }
                        } else {
                            let a2222 = v2;
                            let a2223 = 34u64;
                            if nat_lt(a2222, a2223) {
                                Ok(4u64)
                            } else {
                                Ok(10u64)
                            }
                        }
                    }
                } else {
                    let a2224 = v2;
                    let a2225 = 38u64;
                    if nat_lt(a2224, a2225) {
                        let a2226 = v2;
                        let a2227 = 36u64;
                        if nat_lt(a2226, a2227) {
                            Ok(12u64)
                        } else {
                            let a2228 = v2;
                            let a2229 = 37u64;
                            if nat_lt(a2228, a2229) {
                                Ok(13u64)
                            } else {
                                Ok(2u64)
                            }
                        }
                    } else {
                        let a2230 = v2;
                        let a2231 = 40u64;
                        if nat_lt(a2230, a2231) {
                            let a2232 = v2;
                            let a2233 = 39u64;
                            if nat_lt(a2232, a2233) {
                                Ok(7u64)
                            } else {
                                Ok(14u64)
                            }
                        } else {
                            let a2234 = v2;
                            let a2235 = 41u64;
                            if nat_lt(a2234, a2235) {
                                Ok(6u64)
                            } else {
                                Ok(5u64)
                            }
                        }
                    }
                }
            } else {
                let a2236 = v2;
                let a2237 = 49u64;
                if nat_lt(a2236, a2237) {
                    let a2238 = v2;
                    let a2239 = 45u64;
                    if nat_lt(a2238, a2239) {
                        let a2240 = v2;
                        let a2241 = 43u64;
                        if nat_lt(a2240, a2241) {
                            Ok(9u64)
                        } else {
                            let a2242 = v2;
                            let a2243 = 44u64;
                            if nat_lt(a2242, a2243) {
                                Ok(0u64)
                            } else {
                                Ok(11u64)
                            }
                        }
                    } else {
                        let a2244 = v2;
                        let a2245 = 47u64;
                        if nat_lt(a2244, a2245) {
                            let a2246 = v2;
                            let a2247 = 46u64;
                            if nat_lt(a2246, a2247) {
                                Ok(15u64)
                            } else {
                                Ok(8u64)
                            }
                        } else {
                            let a2248 = v2;
                            let a2249 = 48u64;
                            if nat_lt(a2248, a2249) {
                                Ok(1u64)
                            } else {
                                Ok(10u64)
                            }
                        }
                    }
                } else {
                    let a2250 = v2;
                    let a2251 = 52u64;
                    if nat_lt(a2250, a2251) {
                        let a2252 = v2;
                        let a2253 = 50u64;
                        if nat_lt(a2252, a2253) {
                            Ok(7u64)
                        } else {
                            let a2254 = v2;
                            let a2255 = 51u64;
                            if nat_lt(a2254, a2255) {
                                Ok(12u64)
                            } else {
                                Ok(9u64)
                            }
                        }
                    } else {
                        let a2256 = v2;
                        let a2257 = 54u64;
                        if nat_lt(a2256, a2257) {
                            let a2258 = v2;
                            let a2259 = 53u64;
                            if nat_lt(a2258, a2259) {
                                Ok(14u64)
                            } else {
                                Ok(3u64)
                            }
                        } else {
                            let a2260 = v2;
                            let a2261 = 55u64;
                            if nat_lt(a2260, a2261) {
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
        let a2262 = v2;
        let a2263 = 84u64;
        if nat_lt(a2262, a2263) {
            let a2264 = v2;
            let a2265 = 70u64;
            if nat_lt(a2264, a2265) {
                let a2266 = v2;
                let a2267 = 63u64;
                if nat_lt(a2266, a2267) {
                    let a2268 = v2;
                    let a2269 = 59u64;
                    if nat_lt(a2268, a2269) {
                        let a2270 = v2;
                        let a2271 = 57u64;
                        if nat_lt(a2270, a2271) {
                            Ok(4u64)
                        } else {
                            let a2272 = v2;
                            let a2273 = 58u64;
                            if nat_lt(a2272, a2273) {
                                Ok(0u64)
                            } else {
                                Ok(11u64)
                            }
                        }
                    } else {
                        let a2274 = v2;
                        let a2275 = 61u64;
                        if nat_lt(a2274, a2275) {
                            let a2276 = v2;
                            let a2277 = 60u64;
                            if nat_lt(a2276, a2277) {
                                Ok(2u64)
                            } else {
                                Ok(5u64)
                            }
                        } else {
                            let a2278 = v2;
                            let a2279 = 62u64;
                            if nat_lt(a2278, a2279) {
                                Ok(8u64)
                            } else {
                                Ok(1u64)
                            }
                        }
                    }
                } else {
                    let a2280 = v2;
                    let a2281 = 66u64;
                    if nat_lt(a2280, a2281) {
                        let a2282 = v2;
                        let a2283 = 64u64;
                        if nat_lt(a2282, a2283) {
                            Ok(6u64)
                        } else {
                            let a2284 = v2;
                            let a2285 = 65u64;
                            if nat_lt(a2284, a2285) {
                                Ok(12u64)
                            } else {
                                Ok(13u64)
                            }
                        }
                    } else {
                        let a2286 = v2;
                        let a2287 = 68u64;
                        if nat_lt(a2286, a2287) {
                            let a2288 = v2;
                            let a2289 = 67u64;
                            if nat_lt(a2288, a2289) {
                                Ok(9u64)
                            } else {
                                Ok(11u64)
                            }
                        } else {
                            let a2290 = v2;
                            let a2291 = 69u64;
                            if nat_lt(a2290, a2291) {
                                Ok(15u64)
                            } else {
                                Ok(10u64)
                            }
                        }
                    }
                }
            } else {
                let a2292 = v2;
                let a2293 = 77u64;
                if nat_lt(a2292, a2293) {
                    let a2294 = v2;
                    let a2295 = 73u64;
                    if nat_lt(a2294, a2295) {
                        let a2296 = v2;
                        let a2297 = 71u64;
                        if nat_lt(a2296, a2297) {
                            Ok(14u64)
                        } else {
                            let a2298 = v2;
                            let a2299 = 72u64;
                            if nat_lt(a2298, a2299) {
                                Ok(8u64)
                            } else {
                                Ok(7u64)
                            }
                        }
                    } else {
                        let a2300 = v2;
                        let a2301 = 75u64;
                        if nat_lt(a2300, a2301) {
                            let a2302 = v2;
                            let a2303 = 74u64;
                            if nat_lt(a2302, a2303) {
                                Ok(2u64)
                            } else {
                                Ok(5u64)
                            }
                        } else {
                            let a2304 = v2;
                            let a2305 = 76u64;
                            if nat_lt(a2304, a2305) {
                                Ok(3u64)
                            } else {
                                Ok(0u64)
                            }
                        }
                    }
                } else {
                    let a2306 = v2;
                    let a2307 = 80u64;
                    if nat_lt(a2306, a2307) {
                        let a2308 = v2;
                        let a2309 = 78u64;
                        if nat_lt(a2308, a2309) {
                            Ok(1u64)
                        } else {
                            let a2310 = v2;
                            let a2311 = 79u64;
                            if nat_lt(a2310, a2311) {
                                Ok(6u64)
                            } else {
                                Ok(4u64)
                            }
                        }
                    } else {
                        let a2312 = v2;
                        let a2313 = 82u64;
                        if nat_lt(a2312, a2313) {
                            let a2314 = v2;
                            let a2315 = 81u64;
                            if nat_lt(a2314, a2315) {
                                Ok(9u64)
                            } else {
                                Ok(14u64)
                            }
                        } else {
                            let a2316 = v2;
                            let a2317 = 83u64;
                            if nat_lt(a2316, a2317) {
                                Ok(11u64)
                            } else {
                                Ok(5u64)
                            }
                        }
                    }
                }
            }
        } else {
            let a2318 = v2;
            let a2319 = 98u64;
            if nat_lt(a2318, a2319) {
                let a2320 = v2;
                let a2321 = 91u64;
                if nat_lt(a2320, a2321) {
                    let a2322 = v2;
                    let a2323 = 87u64;
                    if nat_lt(a2322, a2323) {
                        let a2324 = v2;
                        let a2325 = 85u64;
                        if nat_lt(a2324, a2325) {
                            Ok(8u64)
                        } else {
                            let a2326 = v2;
                            let a2327 = 86u64;
                            if nat_lt(a2326, a2327) {
                                Ok(12u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    } else {
                        let a2328 = v2;
                        let a2329 = 89u64;
                        if nat_lt(a2328, a2329) {
                            let a2330 = v2;
                            let a2331 = 88u64;
                            if nat_lt(a2330, a2331) {
                                Ok(1u64)
                            } else {
                                Ok(13u64)
                            }
                        } else {
                            let a2332 = v2;
                            let a2333 = 90u64;
                            if nat_lt(a2332, a2333) {
                                Ok(3u64)
                            } else {
                                Ok(0u64)
                            }
                        }
                    }
                } else {
                    let a2334 = v2;
                    let a2335 = 94u64;
                    if nat_lt(a2334, a2335) {
                        let a2336 = v2;
                        let a2337 = 92u64;
                        if nat_lt(a2336, a2337) {
                            Ok(10u64)
                        } else {
                            let a2338 = v2;
                            let a2339 = 93u64;
                            if nat_lt(a2338, a2339) {
                                Ok(2u64)
                            } else {
                                Ok(6u64)
                            }
                        }
                    } else {
                        let a2340 = v2;
                        let a2341 = 96u64;
                        if nat_lt(a2340, a2341) {
                            let a2342 = v2;
                            let a2343 = 95u64;
                            if nat_lt(a2342, a2343) {
                                Ok(4u64)
                            } else {
                                Ok(7u64)
                            }
                        } else {
                            let a2344 = v2;
                            let a2345 = 97u64;
                            if nat_lt(a2344, a2345) {
                                Ok(11u64)
                            } else {
                                Ok(15u64)
                            }
                        }
                    }
                }
            } else {
                let a2346 = v2;
                let a2347 = 105u64;
                if nat_lt(a2346, a2347) {
                    let a2348 = v2;
                    let a2349 = 101u64;
                    if nat_lt(a2348, a2349) {
                        let a2350 = v2;
                        let a2351 = 99u64;
                        if nat_lt(a2350, a2351) {
                            Ok(5u64)
                        } else {
                            let a2352 = v2;
                            let a2353 = 100u64;
                            if nat_lt(a2352, a2353) {
                                Ok(0u64)
                            } else {
                                Ok(1u64)
                            }
                        }
                    } else {
                        let a2354 = v2;
                        let a2355 = 103u64;
                        if nat_lt(a2354, a2355) {
                            let a2356 = v2;
                            let a2357 = 102u64;
                            if nat_lt(a2356, a2357) {
                                Ok(9u64)
                            } else {
                                Ok(8u64)
                            }
                        } else {
                            let a2358 = v2;
                            let a2359 = 104u64;
                            if nat_lt(a2358, a2359) {
                                Ok(6u64)
                            } else {
                                Ok(14u64)
                            }
                        }
                    }
                } else {
                    let a2360 = v2;
                    let a2361 = 108u64;
                    if nat_lt(a2360, a2361) {
                        let a2362 = v2;
                        let a2363 = 106u64;
                        if nat_lt(a2362, a2363) {
                            Ok(10u64)
                        } else {
                            let a2364 = v2;
                            let a2365 = 107u64;
                            if nat_lt(a2364, a2365) {
                                Ok(2u64)
                            } else {
                                Ok(12u64)
                            }
                        }
                    } else {
                        let a2366 = v2;
                        let a2367 = 110u64;
                        if nat_lt(a2366, a2367) {
                            let a2368 = v2;
                            let a2369 = 109u64;
                            if nat_lt(a2368, a2369) {
                                Ok(3u64)
                            } else {
                                Ok(4u64)
                            }
                        } else {
                            let a2370 = v2;
                            let a2371 = 111u64;
                            if nat_lt(a2370, a2371) {
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

pub fn f75(v0: u64) -> u32 {
    let a2372 = v0;
    let a2373 = 12u64;
    if nat_lt(a2372, a2373) {
        let a2374 = v0;
        let a2375 = 9u64;
        if nat_lt(a2374, a2375) {
            let a2376 = v0;
            let a2377 = 8u64;
            if nat_lt(a2376, a2377) {
                7u32
            } else {
                8u32
            }
        } else {
            let a2378 = v0;
            let a2379 = 10u64;
            if nat_lt(a2378, a2379) {
                9u32
            } else {
                let a2380 = v0;
                let a2381 = 11u64;
                if nat_lt(a2380, a2381) {
                    10u32
                } else {
                    11u32
                }
            }
        }
    } else {
        let a2382 = v0;
        let a2383 = 14u64;
        if nat_lt(a2382, a2383) {
            let a2384 = v0;
            let a2385 = 13u64;
            if nat_lt(a2384, a2385) {
                12u32
            } else {
                13u32
            }
        } else {
            let a2386 = v0;
            let a2387 = 15u64;
            if nat_lt(a2386, a2387) {
                14u32
            } else {
                let a2388 = v0;
                let a2389 = 16u64;
                if nat_lt(a2388, a2389) {
                    15u32
                } else {
                    16u32
                }
            }
        }
    }
}

pub fn f76(v0: Adt1, v1: u64) -> u32 {
    let a2390 = v1;
    let a2391 = 4u64;
    if nat_lt(a2390, a2391) {
        let a2392 = v1;
        let a2393 = 2u64;
        if nat_lt(a2392, a2393) {
            let a2394 = v1;
            let a2395 = 1u64;
            if nat_lt(a2394, a2395) {
                match v0 {
                    Adt1::C0(h2396, _, _, _, _, _, _, _) => {
                        h2396
                    }
                }
            } else {
                match v0 {
                    Adt1::C0(_, h2397, _, _, _, _, _, _) => {
                        h2397
                    }
                }
            }
        } else {
            let a2398 = v1;
            let a2399 = 3u64;
            if nat_lt(a2398, a2399) {
                match v0 {
                    Adt1::C0(_, _, h2400, _, _, _, _, _) => {
                        h2400
                    }
                }
            } else {
                match v0 {
                    Adt1::C0(_, _, _, h2401, _, _, _, _) => {
                        h2401
                    }
                }
            }
        }
    } else {
        let a2402 = v1;
        let a2403 = 6u64;
        if nat_lt(a2402, a2403) {
            let a2404 = v1;
            let a2405 = 5u64;
            if nat_lt(a2404, a2405) {
                match v0 {
                    Adt1::C0(_, _, _, _, h2406, _, _, _) => {
                        h2406
                    }
                }
            } else {
                match v0 {
                    Adt1::C0(_, _, _, _, _, h2407, _, _) => {
                        h2407
                    }
                }
            }
        } else {
            let a2408 = v1;
            let a2409 = 7u64;
            if nat_lt(a2408, a2409) {
                match v0 {
                    Adt1::C0(_, _, _, _, _, _, h2410, _) => {
                        h2410
                    }
                }
            } else {
                match v0 {
                    Adt1::C0(_, _, _, _, _, _, _, h2411) => {
                        h2411
                    }
                }
            }
        }
    }
}

pub fn f77(v0: Adt11, v1: u32, v2: u32) -> u32 {
    let m2412 = v0;
    match m2412 {
        Adt11::C0 => {
            f78(v1, v2)
        }
        Adt11::C1 => {
            f78(v1, v2)
        }
        Adt11::C2 => {
            f79(v1, v2)
        }
        Adt11::C3 => {
            f80(v1, v2)
        }
        Adt11::C4 => {
            f79(v1, v2)
        }
    }
}

pub fn f78(v0: u32, v1: u32) -> u32 {
    let m2415 = {
        let a2413 = f81(v0);
        let a2414 = f81(v1);
        fixed_u64::checked_add(a2413, a2414)
    };
    match m2415 {
        None => {
            0u32
        }
        Some(v2) => {
            f82(v2)
        }
    }
}

pub fn f79(v0: u32, v1: u32) -> u32 {
    let a2416 = v0;
    let a2417 = v1;
    fixed_u32::bit_xor(a2416, a2417)
}

pub fn f80(v0: u32, v1: u32) -> u32 {
    let m2420 = {
        let a2418 = 32u32;
        let a2419 = v1;
        fixed_u32::checked_sub(a2418, a2419)
    };
    match m2420 {
        None => {
            v0
        }
        Some(v2) => {
            f83(v0, v1, v2)
        }
    }
}

pub fn f81(v0: u32) -> u64 {
    let m2422 = {
        let a2421 = v0;
        fixed_u64::convert(i128::from(a2421))
    };
    match m2422 {
        None => {
            0u64
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f82(v0: u64) -> u32 {
    let m2426 = {
        let a2425 = {
            let a2423 = v0;
            let a2424 = 4294967295u64;
            fixed_u64::bit_and(a2423, a2424)
        };
        fixed_u32::convert(i128::from(a2425))
    };
    match m2426 {
        None => {
            0u32
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f83(v0: u32, v1: u32, v2: u32) -> u32 {
    let a2427 = f52(v0, v1);
    let a2428 = f64(v0, v2);
    fixed_u32::bit_or(a2427, a2428)
}

pub fn run(p0: Bytes, p1: u64) -> R<Bytes> {
    f0(p0, p1)
}
