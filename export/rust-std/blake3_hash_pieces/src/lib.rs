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
    C0(Adt3, u32, u64, Bytes, List<Adt3>),
}

#[derive(Clone)]
pub enum Adt3 {
    C0(u32, u32, u32, u32, u32, u32, u32, u32),
}

#[derive(Clone)]
pub enum Adt4 {
    C0(Adt3, Adt5, u64, u32, u32),
}

#[derive(Clone)]
pub enum Adt5 {
    C0(u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32),
}

#[derive(Clone)]
pub enum Adt6 {
    C0(u32, u32, u32, u32),
}

#[derive(Clone)]
pub enum Fn0 {
    F4,
}

impl Fn0 {
    pub fn apply(&self, p0: Adt2, p1: Bytes) -> R<Adt2> {
        match self {
            Fn0::F4 => f4(p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn1 {
    F19(Adt4),
}

impl Fn1 {
    pub fn apply(&self, p0: (u64, Bytes)) -> R<(u64, Bytes)> {
        match self {
            Fn1::F19(k0) => f19(k0.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn2 {
    F21(u32, Adt3),
}

impl Fn2 {
    pub fn apply(&self, p0: Adt4, p1: Adt3) -> Adt4 {
        match self {
            Fn2::F21(k0, k1) => f21(*k0, k1.clone(), p0, p1),
        }
    }
}

#[derive(Clone)]
pub enum Fn3 {
    F24(u32, Bytes, Adt3, Adt2),
}

impl Fn3 {
    pub fn apply(&self, p0: (u64, List<Adt3>)) -> R<(u64, List<Adt3>)> {
        match self {
            Fn3::F24(k0, k1, k2, k3) => f24(*k0, k1.clone(), k2.clone(), k3.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn4 {
    F35(u32, u64, u32, u64, u32, Bytes),
}

impl Fn4 {
    pub fn apply(&self, p0: (u64, Adt3)) -> R<(u64, Adt3)> {
        match self {
            Fn4::F35(k0, k1, k2, k3, k4, k5) => f35(*k0, *k1, *k2, *k3, *k4, k5.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn5 {
    F43,
}

impl Fn5 {
    pub fn apply(&self, p0: (Adt5, Adt5)) -> (Adt5, Adt5) {
        match self {
            Fn5::F43 => f43(p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn6 {
    F48,
}

impl Fn6 {
    pub fn apply(&self, p0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
        match self {
            Fn6::F48 => f48(p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn7 {
    F51(u32, Adt3),
}

impl Fn7 {
    pub fn apply(&self, p0: (Adt3, (u64, List<Adt3>))) -> Option<(Adt3, (u64, List<Adt3>))> {
        match self {
            Fn7::F51(k0, k1) => f51(*k0, k1.clone(), p0),
        }
    }
}

pub fn f0(v0: Adt0, v1: List<Bytes>, v2: u64) -> R<Result<Bytes, Adt1>> {
    let m1 = f1(v0)?;
    match m1 {
        Ok(v3) => {
            Ok(Ok::<Bytes, Adt1>(f2(f3(Fn0::F4, v3, v1)?, v2)?))
        }
        Err(v4) => {
            Ok(Err::<Bytes, Adt1>(v4))
        }
    }
}

pub fn f1(v0: Adt0) -> R<Result<Adt2, Adt1>> {
    let m2 = v0;
    match m2 {
        Adt0::C0 => {
            Ok(Ok::<Adt2, Adt1>(f5(f6(), 0u32)))
        }
        Adt0::C1(v1) => {
            let a4 = {
                let a3 = v1.clone();
                length_bytes(a3)
            };
            let a5 = 32u64;
            if nat_eq(a4, a5) {
                Ok(Ok::<Adt2, Adt1>(f5(f7(v1)?, f8())))
            } else {
                Ok(Err::<Adt2, Adt1>(Adt1::C0({
                    let a6 = v1;
                    length_bytes(a6)
                })))
            }
        }
        Adt0::C2(v2) => {
            Ok(Ok::<Adt2, Adt1>(f5(f9(v2)?, f10())))
        }
    }
}

pub fn f2(v0: Adt2, v1: u64) -> R<Bytes> {
    f11(f12(v0)?, v1)
}

pub fn f3(v0: Fn0, p10: Adt2, p11: List<Bytes>) -> R<Adt2> {
    let m9 = (p10, p11);
    {
        let mut m9 = m9;
        loop {
            let step = {
                let (v1, v2) = m9;
                let m7 = v2;
                match m7.uncons() {
                    None => {
                        Err::<(Adt2, List<Bytes>), R<Adt2>>(Ok(v1))
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt2, List<Bytes>), R<Adt2>>(({
                            let c8: Fn0 = v0.clone();
                            c8.apply(v1, v3)?
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m9 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f4(v0: Adt2, v1: Bytes) -> R<Adt2> {
    f13(v0, v1)
}

pub fn f5(v0: Adt3, v1: u32) -> Adt2 {
    Adt2::C0(v0, v1, 0u64, Bytes::lit(&[]), List::<Adt3>::nil())
}

pub fn f6() -> Adt3 {
    Adt3::C0(1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, 1359893119u32, 2600822924u32, 528734635u32, 1541459225u32)
}

pub fn f7(v0: Bytes) -> R<Adt3> {
    Ok(f14(f15(v0.clone(), {
        let a12 = v0;
        length_bytes(a12)
    }, 0u64)?))
}

pub fn f8() -> u32 {
    16u32
}

pub fn f9(v0: Bytes) -> R<Adt3> {
    Ok(f14(f16(f12(f13(f5(f6(), f17()), v0)?)?, 0u64)))
}

pub fn f10() -> u32 {
    64u32
}

pub fn f11(v0: Adt4, v1: u64) -> R<Bytes> {
    let v2: u64 = {
        let a15 = {
            let a13 = v1;
            let a14 = 63u64;
            nat_add(a13, a14)?
        };
        let a16 = 64u64;
        let a17 = 0u64;
        nat_quot(a15, a16, a17)
    };
    let v3: (u64, Bytes) = f18(Fn1::F19(v0), v2, (0u64, Bytes::lit(&[])))?;
    let m22 = {
        let a19 = {
            let (_, h18) = v3.clone();
            h18
        };
        let a20 = 0u64;
        let a21 = v1;
        slice_bytes(a19, a20, a21)
    };
    match m22 {
        None => {
            let (_, h23) = v3;
            Ok(h23)
        }
        Some(v4) => {
            Ok(v4)
        }
    }
}

pub fn f12(v0: Adt2) -> R<Adt4> {
    let v1: Adt3 = match v0.clone() {
        Adt2::C0(h24, _, _, _, _) => {
            h24
        }
    };
    let v2: u32 = match v0.clone() {
        Adt2::C0(_, h25, _, _, _) => {
            h25
        }
    };
    let v3: Bytes = match v0.clone() {
        Adt2::C0(_, _, _, h26, _) => {
            h26
        }
    };
    Ok(f20(Fn2::F21(v2, v1.clone()), f22(v1, v2, v3.clone(), 0u64, {
        let a27 = v3;
        length_bytes(a27)
    }, match v0.clone() {
        Adt2::C0(_, _, h28, _, _) => {
            h28
        }
    })?, match v0 {
        Adt2::C0(_, _, _, _, h29) => {
            h29
        }
    }))
}

pub fn f13(v0: Adt2, v1: Bytes) -> R<Adt2> {
    let v2: Bytes = {
        let a31 = match v0.clone() {
            Adt2::C0(_, _, _, h30, _) => {
                h30
            }
        };
        let a32 = v1;
        append_bytes(a31, a32)
    };
    let v3: u64 = {
        let a33 = v2.clone();
        length_bytes(a33)
    };
    let v4: u64 = {
        let a34 = v3;
        let a35 = 0u64;
        if nat_eq(a34, a35) {
            0u64
        } else {
            let a38 = {
                let a36 = v3;
                let a37 = 1u64;
                nat_sub(a36, a37)
            };
            let a39 = 1024u64;
            let a40 = 0u64;
            nat_quot(a38, a39, a40)
        }
    };
    let v5: u32 = match v0.clone() {
        Adt2::C0(_, h41, _, _, _) => {
            h41
        }
    };
    let v6: Adt3 = match v0.clone() {
        Adt2::C0(h42, _, _, _, _) => {
            h42
        }
    };
    let v7: (u64, List<Adt3>) = f23(Fn3::F24(v5, v2.clone(), v6.clone(), v0.clone()), v4, (0u64, match v0.clone() {
        Adt2::C0(_, _, _, _, h43) => {
            h43
        }
    }))?;
    let v8: u64 = {
        let a44 = v4;
        let a45 = 1024u64;
        nat_mul(a44, a45)?
    };
    Ok(Adt2::C0(v6, v5, {
        let a47 = match v0 {
            Adt2::C0(_, _, h46, _, _) => {
                h46
            }
        };
        let a48 = v4;
        nat_add(a47, a48)?
    }, {
        let m54 = {
            let a51 = v2;
            let a52 = v8;
            let a53 = {
                let a49 = v3;
                let a50 = v8;
                nat_sub(a49, a50)
            };
            slice_bytes(a51, a52, a53)
        };
        match m54 {
            None => {
                Bytes::lit(&[])
            }
            Some(v9) => {
                v9
            }
        }
    }, {
        let (_, h55) = v7;
        h55
    }))
}

pub fn f14(v0: Adt5) -> Adt3 {
    Adt3::C0(match v0.clone() {
        Adt5::C0(h56, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h56
        }
    }, match v0.clone() {
        Adt5::C0(_, h57, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h57
        }
    }, match v0.clone() {
        Adt5::C0(_, _, h58, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h58
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, h59, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h59
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, h60, _, _, _, _, _, _, _, _, _, _, _) => {
            h60
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, h61, _, _, _, _, _, _, _, _, _, _) => {
            h61
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, h62, _, _, _, _, _, _, _, _, _) => {
            h62
        }
    }, match v0 {
        Adt5::C0(_, _, _, _, _, _, _, h63, _, _, _, _, _, _, _, _) => {
            h63
        }
    })
}

pub fn f15(v0: Bytes, v1: u64, v2: u64) -> R<Adt5> {
    Ok(Adt5::C0(f25(v0.clone(), v1, {
        let a64 = v2;
        let a65 = 0u64;
        nat_add(a64, a65)?
    })?, f25(v0.clone(), v1, {
        let a66 = v2;
        let a67 = 4u64;
        nat_add(a66, a67)?
    })?, f25(v0.clone(), v1, {
        let a68 = v2;
        let a69 = 8u64;
        nat_add(a68, a69)?
    })?, f25(v0.clone(), v1, {
        let a70 = v2;
        let a71 = 12u64;
        nat_add(a70, a71)?
    })?, f25(v0.clone(), v1, {
        let a72 = v2;
        let a73 = 16u64;
        nat_add(a72, a73)?
    })?, f25(v0.clone(), v1, {
        let a74 = v2;
        let a75 = 20u64;
        nat_add(a74, a75)?
    })?, f25(v0.clone(), v1, {
        let a76 = v2;
        let a77 = 24u64;
        nat_add(a76, a77)?
    })?, f25(v0.clone(), v1, {
        let a78 = v2;
        let a79 = 28u64;
        nat_add(a78, a79)?
    })?, f25(v0.clone(), v1, {
        let a80 = v2;
        let a81 = 32u64;
        nat_add(a80, a81)?
    })?, f25(v0.clone(), v1, {
        let a82 = v2;
        let a83 = 36u64;
        nat_add(a82, a83)?
    })?, f25(v0.clone(), v1, {
        let a84 = v2;
        let a85 = 40u64;
        nat_add(a84, a85)?
    })?, f25(v0.clone(), v1, {
        let a86 = v2;
        let a87 = 44u64;
        nat_add(a86, a87)?
    })?, f25(v0.clone(), v1, {
        let a88 = v2;
        let a89 = 48u64;
        nat_add(a88, a89)?
    })?, f25(v0.clone(), v1, {
        let a90 = v2;
        let a91 = 52u64;
        nat_add(a90, a91)?
    })?, f25(v0.clone(), v1, {
        let a92 = v2;
        let a93 = 56u64;
        nat_add(a92, a93)?
    })?, f25(v0, v1, {
        let a94 = v2;
        let a95 = 60u64;
        nat_add(a94, a95)?
    })?))
}

pub fn f16(v0: Adt4, v1: u64) -> Adt5 {
    f26(match v0.clone() {
        Adt4::C0(h96, _, _, _, _) => {
            h96
        }
    }, match v0.clone() {
        Adt4::C0(_, h97, _, _, _) => {
            h97
        }
    }, f27(v1), f28(v1), match v0.clone() {
        Adt4::C0(_, _, _, h98, _) => {
            h98
        }
    }, {
        let a100 = match v0 {
            Adt4::C0(_, _, _, _, h99) => {
                h99
            }
        };
        let a101 = f29();
        fixed_u32::bit_or(a100, a101)
    })
}

pub fn f17() -> u32 {
    32u32
}

pub fn f18(v0: Fn1, p105: u64, p106: (u64, Bytes)) -> R<(u64, Bytes)> {
    let m104 = (p105, p106);
    {
        let mut m104 = m104;
        loop {
            let step = {
                let (v1, v2) = m104;
                let m102 = v1;
                if m102 == 0 {
                    Err::<(u64, (u64, Bytes)), R<(u64, Bytes)>>(Ok(v2))
                } else {
                    let v3 = m102 - 1;
                    Ok::<(u64, (u64, Bytes)), R<(u64, Bytes)>>((v3, {
                        let c103: Fn1 = v0.clone();
                        c103.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m104 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f19(v0: Adt4, v1: (u64, Bytes)) -> R<(u64, Bytes)> {
    Ok(({
        let a108 = {
            let (h107, _) = v1.clone();
            h107
        };
        let a109 = 1u64;
        nat_add(a108, a109)?
    }, {
        let a112 = {
            let (_, h110) = v1;
            h110
        };
        let a113 = f30(f16(v0, {
            let (h111, _) = v1;
            h111
        }));
        append_bytes(a112, a113)
    }))
}

pub fn f20(v0: Fn2, p117: Adt4, p118: List<Adt3>) -> Adt4 {
    let m116 = (p117, p118);
    {
        let mut m116 = m116;
        loop {
            let step = {
                let (v1, v2) = m116;
                let m114 = v2;
                match m114.uncons() {
                    None => {
                        Err::<(Adt4, List<Adt3>), Adt4>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt4, List<Adt3>), Adt4>(({
                            let c115: Fn2 = v0.clone();
                            c115.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m116 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f21(v0: u32, v1: Adt3, v2: Adt4, v3: Adt3) -> Adt4 {
    f31(v3, f32(v2), v1, v0)
}

pub fn f22(v0: Adt3, v1: u32, v2: Bytes, v3: u64, v4: u64, v5: u64) -> R<Adt4> {
    let v6: u64 = {
        let a119 = v3;
        let a120 = v4;
        nat_add(a119, a120)?
    };
    let v7: u64 = f33(v4)?;
    let v8: u32 = f27(v5);
    let v9: u32 = f28(v5);
    let v10: (u64, Adt3) = f34(Fn4::F35(v1, v3, v9, v6, v8, v2.clone()), {
        let a121 = v7;
        let a122 = 64u64;
        let a123 = 0u64;
        nat_quot(a121, a122, a123)
    }, (0u64, v0))?;
    Ok(Adt4::C0({
        let (_, h124) = v10;
        h124
    }, f15(v2, v6, {
        let a125 = v3;
        let a126 = v7;
        nat_add(a125, a126)?
    })?, v5, f36({
        let a127 = v4;
        let a128 = v7;
        nat_sub(a127, a128)
    }, 7u64), {
        let a131 = v1;
        let a132 = {
            let a129 = f37();
            let a130 = f38(v7);
            fixed_u32::bit_or(a129, a130)
        };
        fixed_u32::bit_or(a131, a132)
    }))
}

pub fn f23(v0: Fn3, p136: u64, p137: (u64, List<Adt3>)) -> R<(u64, List<Adt3>)> {
    let m135 = (p136, p137);
    {
        let mut m135 = m135;
        loop {
            let step = {
                let (v1, v2) = m135;
                let m133 = v1;
                if m133 == 0 {
                    Err::<(u64, (u64, List<Adt3>)), R<(u64, List<Adt3>)>>(Ok(v2))
                } else {
                    let v3 = m133 - 1;
                    Ok::<(u64, (u64, List<Adt3>)), R<(u64, List<Adt3>)>>((v3, {
                        let c134: Fn3 = v0.clone();
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

pub fn f24(v0: u32, v1: Bytes, v2: Adt3, v3: Adt2, v4: (u64, List<Adt3>)) -> R<(u64, List<Adt3>)> {
    let v5: u64 = {
        let a140 = match v3 {
            Adt2::C0(_, _, h138, _, _) => {
                h138
            }
        };
        let a141 = {
            let (h139, _) = v4.clone();
            h139
        };
        nat_add(a140, a141)?
    };
    Ok(({
        let a143 = {
            let (h142, _) = v4.clone();
            h142
        };
        let a144 = 1u64;
        nat_add(a143, a144)?
    }, f39(v2.clone(), v0, {
        let (_, h145) = v4;
        h145
    }, f32(f22(v2, v0, v1, {
        let a147 = {
            let (h146, _) = v4;
            h146
        };
        let a148 = 1024u64;
        nat_mul(a147, a148)?
    }, 1024u64, v5)?), {
        let a149 = v5;
        let a150 = 1u64;
        nat_add(a149, a150)?
    })))
}

pub fn f25(v0: Bytes, v1: u64, v2: u64) -> R<u32> {
    Ok(f40(f41(v0.clone(), v1, v2), f41(v0.clone(), v1, {
        let a151 = v2;
        let a152 = 1u64;
        nat_add(a151, a152)?
    }), f41(v0.clone(), v1, {
        let a153 = v2;
        let a154 = 2u64;
        nat_add(a153, a154)?
    }), f41(v0, v1, {
        let a155 = v2;
        let a156 = 3u64;
        nat_add(a155, a156)?
    })))
}

pub fn f26(v0: Adt3, v1: Adt5, v2: u32, v3: u32, v4: u32, v5: u32) -> Adt5 {
    let v6: Adt5 = {
        let (h157, _) = f42(Fn5::F43, 7u64, (Adt5::C0(match v0.clone() {
            Adt3::C0(h158, _, _, _, _, _, _, _) => {
                h158
            }
        }, match v0.clone() {
            Adt3::C0(_, h159, _, _, _, _, _, _) => {
                h159
            }
        }, match v0.clone() {
            Adt3::C0(_, _, h160, _, _, _, _, _) => {
                h160
            }
        }, match v0.clone() {
            Adt3::C0(_, _, _, h161, _, _, _, _) => {
                h161
            }
        }, match v0.clone() {
            Adt3::C0(_, _, _, _, h162, _, _, _) => {
                h162
            }
        }, match v0.clone() {
            Adt3::C0(_, _, _, _, _, h163, _, _) => {
                h163
            }
        }, match v0.clone() {
            Adt3::C0(_, _, _, _, _, _, h164, _) => {
                h164
            }
        }, match v0.clone() {
            Adt3::C0(_, _, _, _, _, _, _, h165) => {
                h165
            }
        }, 1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, v2, v3, v4, v5), v1));
        h157
    };
    Adt5::C0(f44(match v6.clone() {
        Adt5::C0(h166, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h166
        }
    }, match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, h167, _, _, _, _, _, _, _) => {
            h167
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, h168, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h168
        }
    }, match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, h169, _, _, _, _, _, _) => {
            h169
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, h170, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h170
        }
    }, match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, h171, _, _, _, _, _) => {
            h171
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, _, h172, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h172
        }
    }, match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h173, _, _, _, _) => {
            h173
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, _, _, h174, _, _, _, _, _, _, _, _, _, _, _) => {
            h174
        }
    }, match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h175, _, _, _) => {
            h175
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, _, _, _, h176, _, _, _, _, _, _, _, _, _, _) => {
            h176
        }
    }, match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h177, _, _) => {
            h177
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, h178, _, _, _, _, _, _, _, _, _) => {
            h178
        }
    }, match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h179, _) => {
            h179
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, h180, _, _, _, _, _, _, _, _) => {
            h180
        }
    }, match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h181) => {
            h181
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, h182, _, _, _, _, _, _, _) => {
            h182
        }
    }, match v0.clone() {
        Adt3::C0(h183, _, _, _, _, _, _, _) => {
            h183
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, h184, _, _, _, _, _, _) => {
            h184
        }
    }, match v0.clone() {
        Adt3::C0(_, h185, _, _, _, _, _, _) => {
            h185
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, h186, _, _, _, _, _) => {
            h186
        }
    }, match v0.clone() {
        Adt3::C0(_, _, h187, _, _, _, _, _) => {
            h187
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h188, _, _, _, _) => {
            h188
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h189, _, _, _, _) => {
            h189
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h190, _, _, _) => {
            h190
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h191, _, _, _) => {
            h191
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h192, _, _) => {
            h192
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h193, _, _) => {
            h193
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h194, _) => {
            h194
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h195, _) => {
            h195
        }
    }), f44(match v6 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h196) => {
            h196
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, h197) => {
            h197
        }
    }))
}

pub fn f27(v0: u64) -> u32 {
    f36({
        let a198 = v0;
        let a199 = 4294967296u64;
        let a200 = 0u64;
        nat_rem(a198, a199, a200)
    }, 32u64)
}

pub fn f28(v0: u64) -> u32 {
    let a201 = v0;
    let a202 = 4294967296u64;
    if nat_lt(a201, a202) {
        0u32
    } else {
        f36({
            let a203 = v0;
            let a204 = 4294967296u64;
            let a205 = 0u64;
            nat_quot(a203, a204, a205)
        }, 32u64)
    }
}

pub fn f29() -> u32 {
    8u32
}

pub fn f30(v0: Adt5) -> Bytes {
    let a250 = f45(match v0.clone() {
        Adt5::C0(h206, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h206
        }
    });
    let a251 = {
        let a248 = f45(match v0.clone() {
            Adt5::C0(_, h207, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                h207
            }
        });
        let a249 = {
            let a246 = f45(match v0.clone() {
                Adt5::C0(_, _, h208, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                    h208
                }
            });
            let a247 = {
                let a244 = f45(match v0.clone() {
                    Adt5::C0(_, _, _, h209, _, _, _, _, _, _, _, _, _, _, _, _) => {
                        h209
                    }
                });
                let a245 = {
                    let a242 = f45(match v0.clone() {
                        Adt5::C0(_, _, _, _, h210, _, _, _, _, _, _, _, _, _, _, _) => {
                            h210
                        }
                    });
                    let a243 = {
                        let a240 = f45(match v0.clone() {
                            Adt5::C0(_, _, _, _, _, h211, _, _, _, _, _, _, _, _, _, _) => {
                                h211
                            }
                        });
                        let a241 = {
                            let a238 = f45(match v0.clone() {
                                Adt5::C0(_, _, _, _, _, _, h212, _, _, _, _, _, _, _, _, _) => {
                                    h212
                                }
                            });
                            let a239 = {
                                let a236 = f45(match v0.clone() {
                                    Adt5::C0(_, _, _, _, _, _, _, h213, _, _, _, _, _, _, _, _) => {
                                        h213
                                    }
                                });
                                let a237 = {
                                    let a234 = f45(match v0.clone() {
                                        Adt5::C0(_, _, _, _, _, _, _, _, h214, _, _, _, _, _, _, _) => {
                                            h214
                                        }
                                    });
                                    let a235 = {
                                        let a232 = f45(match v0.clone() {
                                            Adt5::C0(_, _, _, _, _, _, _, _, _, h215, _, _, _, _, _, _) => {
                                                h215
                                            }
                                        });
                                        let a233 = {
                                            let a230 = f45(match v0.clone() {
                                                Adt5::C0(_, _, _, _, _, _, _, _, _, _, h216, _, _, _, _, _) => {
                                                    h216
                                                }
                                            });
                                            let a231 = {
                                                let a228 = f45(match v0.clone() {
                                                    Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h217, _, _, _, _) => {
                                                        h217
                                                    }
                                                });
                                                let a229 = {
                                                    let a226 = f45(match v0.clone() {
                                                        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h218, _, _, _) => {
                                                            h218
                                                        }
                                                    });
                                                    let a227 = {
                                                        let a224 = f45(match v0.clone() {
                                                            Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h219, _, _) => {
                                                                h219
                                                            }
                                                        });
                                                        let a225 = {
                                                            let a222 = f45(match v0.clone() {
                                                                Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h220, _) => {
                                                                    h220
                                                                }
                                                            });
                                                            let a223 = f45(match v0 {
                                                                Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h221) => {
                                                                    h221
                                                                }
                                                            });
                                                            append_bytes(a222, a223)
                                                        };
                                                        append_bytes(a224, a225)
                                                    };
                                                    append_bytes(a226, a227)
                                                };
                                                append_bytes(a228, a229)
                                            };
                                            append_bytes(a230, a231)
                                        };
                                        append_bytes(a232, a233)
                                    };
                                    append_bytes(a234, a235)
                                };
                                append_bytes(a236, a237)
                            };
                            append_bytes(a238, a239)
                        };
                        append_bytes(a240, a241)
                    };
                    append_bytes(a242, a243)
                };
                append_bytes(a244, a245)
            };
            append_bytes(a246, a247)
        };
        append_bytes(a248, a249)
    };
    append_bytes(a250, a251)
}

pub fn f31(v0: Adt3, v1: Adt3, v2: Adt3, v3: u32) -> Adt4 {
    Adt4::C0(v2, Adt5::C0(match v0.clone() {
        Adt3::C0(h252, _, _, _, _, _, _, _) => {
            h252
        }
    }, match v0.clone() {
        Adt3::C0(_, h253, _, _, _, _, _, _) => {
            h253
        }
    }, match v0.clone() {
        Adt3::C0(_, _, h254, _, _, _, _, _) => {
            h254
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h255, _, _, _, _) => {
            h255
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h256, _, _, _) => {
            h256
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h257, _, _) => {
            h257
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h258, _) => {
            h258
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, h259) => {
            h259
        }
    }, match v1.clone() {
        Adt3::C0(h260, _, _, _, _, _, _, _) => {
            h260
        }
    }, match v1.clone() {
        Adt3::C0(_, h261, _, _, _, _, _, _) => {
            h261
        }
    }, match v1.clone() {
        Adt3::C0(_, _, h262, _, _, _, _, _) => {
            h262
        }
    }, match v1.clone() {
        Adt3::C0(_, _, _, h263, _, _, _, _) => {
            h263
        }
    }, match v1.clone() {
        Adt3::C0(_, _, _, _, h264, _, _, _) => {
            h264
        }
    }, match v1.clone() {
        Adt3::C0(_, _, _, _, _, h265, _, _) => {
            h265
        }
    }, match v1.clone() {
        Adt3::C0(_, _, _, _, _, _, h266, _) => {
            h266
        }
    }, match v1 {
        Adt3::C0(_, _, _, _, _, _, _, h267) => {
            h267
        }
    }), 0u64, 64u32, {
        let a268 = v3;
        let a269 = f46();
        fixed_u32::bit_or(a268, a269)
    })
}

pub fn f32(v0: Adt4) -> Adt3 {
    f14(f26(match v0.clone() {
        Adt4::C0(h270, _, _, _, _) => {
            h270
        }
    }, match v0.clone() {
        Adt4::C0(_, h271, _, _, _) => {
            h271
        }
    }, f27(match v0.clone() {
        Adt4::C0(_, _, h272, _, _) => {
            h272
        }
    }), f28(match v0.clone() {
        Adt4::C0(_, _, h273, _, _) => {
            h273
        }
    }), match v0.clone() {
        Adt4::C0(_, _, _, h274, _) => {
            h274
        }
    }, match v0 {
        Adt4::C0(_, _, _, _, h275) => {
            h275
        }
    }))
}

pub fn f33(v0: u64) -> R<u64> {
    let a276 = v0;
    let a277 = 0u64;
    if nat_eq(a276, a277) {
        Ok(0u64)
    } else {
        let a283 = {
            let a280 = {
                let a278 = v0;
                let a279 = 1u64;
                nat_sub(a278, a279)
            };
            let a281 = 64u64;
            let a282 = 0u64;
            nat_quot(a280, a281, a282)
        };
        let a284 = 64u64;
        nat_mul(a283, a284)
    }
}

pub fn f34(v0: Fn4, p288: u64, p289: (u64, Adt3)) -> R<(u64, Adt3)> {
    let m287 = (p288, p289);
    {
        let mut m287 = m287;
        loop {
            let step = {
                let (v1, v2) = m287;
                let m285 = v1;
                if m285 == 0 {
                    Err::<(u64, (u64, Adt3)), R<(u64, Adt3)>>(Ok(v2))
                } else {
                    let v3 = m285 - 1;
                    Ok::<(u64, (u64, Adt3)), R<(u64, Adt3)>>((v3, {
                        let c286: Fn4 = v0.clone();
                        c286.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m287 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f35(v0: u32, v1: u64, v2: u32, v3: u64, v4: u32, v5: Bytes, v6: (u64, Adt3)) -> R<(u64, Adt3)> {
    let v7: u64 = {
        let (h290, _) = v6;
        h290
    };
    Ok(({
        let a291 = v7;
        let a292 = 64u64;
        nat_add(a291, a292)?
    }, f14(f26({
        let (_, h293) = v6;
        h293
    }, f15(v5, v3, {
        let a294 = v1;
        let a295 = v7;
        nat_add(a294, a295)?
    })?, v4, v2, 64u32, {
        let a296 = v0;
        let a297 = f38(v7);
        fixed_u32::bit_or(a296, a297)
    }))))
}

pub fn f36(v0: u64, v1: u64) -> u32 {
    let (_, h298) = {
        let (_, h299) = f47(Fn6::F48, v1, (v0, (1u32, 0u32)));
        h299
    };
    h298
}

pub fn f37() -> u32 {
    2u32
}

pub fn f38(v0: u64) -> u32 {
    let a300 = v0;
    let a301 = 0u64;
    if nat_eq(a300, a301) {
        f49()
    } else {
        0u32
    }
}

pub fn f39(v0: Adt3, v1: u32, v2: List<Adt3>, v3: Adt3, v4: u64) -> List<Adt3> {
    let v5: (Adt3, (u64, List<Adt3>)) = {
        let (h302, _) = f50(Fn7::F51(v1, v0), 64u64, (v3, (v4, v2)));
        h302
    };
    List::cons({
        let (h303, _) = v5;
        h303
    }, {
        let (_, h304) = {
            let (_, h305) = v5;
            h305
        };
        h304
    })
}

pub fn f40(v0: u32, v1: u32, v2: u32, v3: u32) -> u32 {
    let a310 = v0;
    let a311 = {
        let a308 = f52(v1, 8u32);
        let a309 = {
            let a306 = f52(v2, 16u32);
            let a307 = f52(v3, 24u32);
            fixed_u32::bit_or(a306, a307)
        };
        fixed_u32::bit_or(a308, a309)
    };
    fixed_u32::bit_or(a310, a311)
}

pub fn f41(v0: Bytes, v1: u64, v2: u64) -> u32 {
    let a312 = v2;
    let a313 = v1;
    if nat_lt(a312, a313) {
        let m316 = {
            let a314 = v0;
            let a315 = v2;
            index_bytes(a314, a315)
        };
        match m316 {
            None => {
                0u32
            }
            Some(v3) => {
                let m318 = {
                    let a317 = v3;
                    fixed_u32::convert(i128::from(a317))
                };
                match m318 {
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

pub fn f42(v0: Fn5, p322: u64, p323: (Adt5, Adt5)) -> (Adt5, Adt5) {
    let m321 = (p322, p323);
    {
        let mut m321 = m321;
        loop {
            let step = {
                let (v1, v2) = m321;
                let m319 = v1;
                if m319 == 0 {
                    Err::<(u64, (Adt5, Adt5)), (Adt5, Adt5)>(v2)
                } else {
                    let v3 = m319 - 1;
                    Ok::<(u64, (Adt5, Adt5)), (Adt5, Adt5)>((v3, {
                        let c320: Fn5 = v0.clone();
                        c320.apply(v2)
                    }))
                }
            };
            match step {
                Ok(next) => { m321 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f43(v0: (Adt5, Adt5)) -> (Adt5, Adt5) {
    (f53({
        let (h324, _) = v0.clone();
        h324
    }, {
        let (_, h325) = v0.clone();
        h325
    }), f54({
        let (_, h326) = v0;
        h326
    }))
}

pub fn f44(v0: u32, v1: u32) -> u32 {
    let a327 = v0;
    let a328 = v1;
    fixed_u32::bit_xor(a327, a328)
}

pub fn f45(v0: u32) -> Bytes {
    let a339 = f55({
        let a329 = v0;
        let a330 = 255u32;
        fixed_u32::bit_and(a329, a330)
    });
    let a340 = {
        let a337 = f55({
            let a331 = f56(v0, 8u32);
            let a332 = 255u32;
            fixed_u32::bit_and(a331, a332)
        });
        let a338 = {
            let a335 = f55({
                let a333 = f56(v0, 16u32);
                let a334 = 255u32;
                fixed_u32::bit_and(a333, a334)
            });
            let a336 = f55(f56(v0, 24u32));
            append_bytes(a335, a336)
        };
        append_bytes(a337, a338)
    };
    append_bytes(a339, a340)
}

pub fn f46() -> u32 {
    4u32
}

pub fn f47(v0: Fn6, p344: u64, p345: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let m343 = (p344, p345);
    {
        let mut m343 = m343;
        loop {
            let step = {
                let (v1, v2) = m343;
                let m341 = v1;
                if m341 == 0 {
                    Err::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>(v2)
                } else {
                    let v3 = m341 - 1;
                    Ok::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>((v3, {
                        let c342: Fn6 = v0.clone();
                        c342.apply(v2)
                    }))
                }
            };
            match step {
                Ok(next) => { m343 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f48(v0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let v1: u64 = {
        let (h346, _) = v0;
        h346
    };
    let v2: u32 = {
        let (h347, _) = {
            let (_, h348) = v0;
            h348
        };
        h347
    };
    let v3: u32 = {
        let (_, h349) = {
            let (_, h350) = v0;
            h350
        };
        h349
    };
    ({
        let a351 = v1;
        let a352 = 2u64;
        let a353 = 0u64;
        nat_quot(a351, a352, a353)
    }, (f52(v2, 1u32), {
        let a357 = {
            let a354 = v1;
            let a355 = 2u64;
            let a356 = 0u64;
            nat_rem(a354, a355, a356)
        };
        let a358 = 1u64;
        if nat_eq(a357, a358) {
            let a359 = v3;
            let a360 = v2;
            fixed_u32::bit_or(a359, a360)
        } else {
            v3
        }
    }))
}

pub fn f49() -> u32 {
    1u32
}

pub fn f50(v0: Fn7, p365: u64, p366: (Adt3, (u64, List<Adt3>))) -> ((Adt3, (u64, List<Adt3>)), bool) {
    let m364 = (p365, p366);
    {
        let mut m364 = m364;
        loop {
            let step = {
                let (v1, v2) = m364;
                let m361 = v1;
                if m361 == 0 {
                    Err::<(u64, (Adt3, (u64, List<Adt3>))), ((Adt3, (u64, List<Adt3>)), bool)>((v2, false))
                } else {
                    let v3 = m361 - 1;
                    let m363 = {
                        let c362: Fn7 = v0.clone();
                        c362.apply(v2.clone())
                    };
                    match m363 {
                        None => {
                            Err::<(u64, (Adt3, (u64, List<Adt3>))), ((Adt3, (u64, List<Adt3>)), bool)>((v2, true))
                        }
                        Some(v4) => {
                            Ok::<(u64, (Adt3, (u64, List<Adt3>))), ((Adt3, (u64, List<Adt3>)), bool)>((v3, v4))
                        }
                    }
                }
            };
            match step {
                Ok(next) => { m364 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f51(v0: u32, v1: Adt3, v2: (Adt3, (u64, List<Adt3>))) -> Option<(Adt3, (u64, List<Adt3>))> {
    f57(v1, v0, v2)
}

pub fn f52(v0: u32, v1: u32) -> u32 {
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

pub fn f53(v0: Adt5, v1: Adt5) -> Adt5 {
    f58(f59(f60(f61(f62(f63(f64(f65(v0, v1.clone()), v1.clone()), v1.clone()), v1.clone()), v1.clone()), v1.clone()), v1.clone()), v1)
}

pub fn f54(v0: Adt5) -> Adt5 {
    Adt5::C0(match v0.clone() {
        Adt5::C0(_, _, h370, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h370
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, h371, _, _, _, _, _, _, _, _, _) => {
            h371
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, h372, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h372
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, h373, _, _, _, _, _) => {
            h373
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, h374, _, _, _, _, _, _, _, _) => {
            h374
        }
    }, match v0.clone() {
        Adt5::C0(h375, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h375
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, h376, _, _, _, _, _, _, _, _, _, _, _) => {
            h376
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h377, _, _) => {
            h377
        }
    }, match v0.clone() {
        Adt5::C0(_, h378, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h378
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h379, _, _, _, _) => {
            h379
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h380, _, _, _) => {
            h380
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, h381, _, _, _, _, _, _, _, _, _, _) => {
            h381
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, h382, _, _, _, _, _, _) => {
            h382
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h383, _) => {
            h383
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h384) => {
            h384
        }
    }, match v0 {
        Adt5::C0(_, _, _, _, _, _, _, _, h385, _, _, _, _, _, _, _) => {
            h385
        }
    })
}

pub fn f55(v0: u32) -> Bytes {
    let a388 = {
        let a386 = v0;
        let a387 = 128u32;
        fixed_u32::bit_and(a386, a387)
    };
    let a389 = 0u32;
    if equal(a388, a389) {
        let a392 = {
            let a390 = v0;
            let a391 = 64u32;
            fixed_u32::bit_and(a390, a391)
        };
        let a393 = 0u32;
        if equal(a392, a393) {
            let a396 = {
                let a394 = v0;
                let a395 = 32u32;
                fixed_u32::bit_and(a394, a395)
            };
            let a397 = 0u32;
            if equal(a396, a397) {
                let a400 = {
                    let a398 = v0;
                    let a399 = 16u32;
                    fixed_u32::bit_and(a398, a399)
                };
                let a401 = 0u32;
                if equal(a400, a401) {
                    let a404 = {
                        let a402 = v0;
                        let a403 = 8u32;
                        fixed_u32::bit_and(a402, a403)
                    };
                    let a405 = 0u32;
                    if equal(a404, a405) {
                        let a408 = {
                            let a406 = v0;
                            let a407 = 4u32;
                            fixed_u32::bit_and(a406, a407)
                        };
                        let a409 = 0u32;
                        if equal(a408, a409) {
                            let a412 = {
                                let a410 = v0;
                                let a411 = 2u32;
                                fixed_u32::bit_and(a410, a411)
                            };
                            let a413 = 0u32;
                            if equal(a412, a413) {
                                let a416 = {
                                    let a414 = v0;
                                    let a415 = 1u32;
                                    fixed_u32::bit_and(a414, a415)
                                };
                                let a417 = 0u32;
                                if equal(a416, a417) {
                                    Bytes::lit(&[0x00])
                                } else {
                                    Bytes::lit(&[0x01])
                                }
                            } else {
                                let a420 = {
                                    let a418 = v0;
                                    let a419 = 1u32;
                                    fixed_u32::bit_and(a418, a419)
                                };
                                let a421 = 0u32;
                                if equal(a420, a421) {
                                    Bytes::lit(&[0x02])
                                } else {
                                    Bytes::lit(&[0x03])
                                }
                            }
                        } else {
                            let a424 = {
                                let a422 = v0;
                                let a423 = 2u32;
                                fixed_u32::bit_and(a422, a423)
                            };
                            let a425 = 0u32;
                            if equal(a424, a425) {
                                let a428 = {
                                    let a426 = v0;
                                    let a427 = 1u32;
                                    fixed_u32::bit_and(a426, a427)
                                };
                                let a429 = 0u32;
                                if equal(a428, a429) {
                                    Bytes::lit(&[0x04])
                                } else {
                                    Bytes::lit(&[0x05])
                                }
                            } else {
                                let a432 = {
                                    let a430 = v0;
                                    let a431 = 1u32;
                                    fixed_u32::bit_and(a430, a431)
                                };
                                let a433 = 0u32;
                                if equal(a432, a433) {
                                    Bytes::lit(&[0x06])
                                } else {
                                    Bytes::lit(&[0x07])
                                }
                            }
                        }
                    } else {
                        let a436 = {
                            let a434 = v0;
                            let a435 = 4u32;
                            fixed_u32::bit_and(a434, a435)
                        };
                        let a437 = 0u32;
                        if equal(a436, a437) {
                            let a440 = {
                                let a438 = v0;
                                let a439 = 2u32;
                                fixed_u32::bit_and(a438, a439)
                            };
                            let a441 = 0u32;
                            if equal(a440, a441) {
                                let a444 = {
                                    let a442 = v0;
                                    let a443 = 1u32;
                                    fixed_u32::bit_and(a442, a443)
                                };
                                let a445 = 0u32;
                                if equal(a444, a445) {
                                    Bytes::lit(&[0x08])
                                } else {
                                    Bytes::lit(&[0x09])
                                }
                            } else {
                                let a448 = {
                                    let a446 = v0;
                                    let a447 = 1u32;
                                    fixed_u32::bit_and(a446, a447)
                                };
                                let a449 = 0u32;
                                if equal(a448, a449) {
                                    Bytes::lit(&[0x0a])
                                } else {
                                    Bytes::lit(&[0x0b])
                                }
                            }
                        } else {
                            let a452 = {
                                let a450 = v0;
                                let a451 = 2u32;
                                fixed_u32::bit_and(a450, a451)
                            };
                            let a453 = 0u32;
                            if equal(a452, a453) {
                                let a456 = {
                                    let a454 = v0;
                                    let a455 = 1u32;
                                    fixed_u32::bit_and(a454, a455)
                                };
                                let a457 = 0u32;
                                if equal(a456, a457) {
                                    Bytes::lit(&[0x0c])
                                } else {
                                    Bytes::lit(&[0x0d])
                                }
                            } else {
                                let a460 = {
                                    let a458 = v0;
                                    let a459 = 1u32;
                                    fixed_u32::bit_and(a458, a459)
                                };
                                let a461 = 0u32;
                                if equal(a460, a461) {
                                    Bytes::lit(&[0x0e])
                                } else {
                                    Bytes::lit(&[0x0f])
                                }
                            }
                        }
                    }
                } else {
                    let a464 = {
                        let a462 = v0;
                        let a463 = 8u32;
                        fixed_u32::bit_and(a462, a463)
                    };
                    let a465 = 0u32;
                    if equal(a464, a465) {
                        let a468 = {
                            let a466 = v0;
                            let a467 = 4u32;
                            fixed_u32::bit_and(a466, a467)
                        };
                        let a469 = 0u32;
                        if equal(a468, a469) {
                            let a472 = {
                                let a470 = v0;
                                let a471 = 2u32;
                                fixed_u32::bit_and(a470, a471)
                            };
                            let a473 = 0u32;
                            if equal(a472, a473) {
                                let a476 = {
                                    let a474 = v0;
                                    let a475 = 1u32;
                                    fixed_u32::bit_and(a474, a475)
                                };
                                let a477 = 0u32;
                                if equal(a476, a477) {
                                    Bytes::lit(&[0x10])
                                } else {
                                    Bytes::lit(&[0x11])
                                }
                            } else {
                                let a480 = {
                                    let a478 = v0;
                                    let a479 = 1u32;
                                    fixed_u32::bit_and(a478, a479)
                                };
                                let a481 = 0u32;
                                if equal(a480, a481) {
                                    Bytes::lit(&[0x12])
                                } else {
                                    Bytes::lit(&[0x13])
                                }
                            }
                        } else {
                            let a484 = {
                                let a482 = v0;
                                let a483 = 2u32;
                                fixed_u32::bit_and(a482, a483)
                            };
                            let a485 = 0u32;
                            if equal(a484, a485) {
                                let a488 = {
                                    let a486 = v0;
                                    let a487 = 1u32;
                                    fixed_u32::bit_and(a486, a487)
                                };
                                let a489 = 0u32;
                                if equal(a488, a489) {
                                    Bytes::lit(&[0x14])
                                } else {
                                    Bytes::lit(&[0x15])
                                }
                            } else {
                                let a492 = {
                                    let a490 = v0;
                                    let a491 = 1u32;
                                    fixed_u32::bit_and(a490, a491)
                                };
                                let a493 = 0u32;
                                if equal(a492, a493) {
                                    Bytes::lit(&[0x16])
                                } else {
                                    Bytes::lit(&[0x17])
                                }
                            }
                        }
                    } else {
                        let a496 = {
                            let a494 = v0;
                            let a495 = 4u32;
                            fixed_u32::bit_and(a494, a495)
                        };
                        let a497 = 0u32;
                        if equal(a496, a497) {
                            let a500 = {
                                let a498 = v0;
                                let a499 = 2u32;
                                fixed_u32::bit_and(a498, a499)
                            };
                            let a501 = 0u32;
                            if equal(a500, a501) {
                                let a504 = {
                                    let a502 = v0;
                                    let a503 = 1u32;
                                    fixed_u32::bit_and(a502, a503)
                                };
                                let a505 = 0u32;
                                if equal(a504, a505) {
                                    Bytes::lit(&[0x18])
                                } else {
                                    Bytes::lit(&[0x19])
                                }
                            } else {
                                let a508 = {
                                    let a506 = v0;
                                    let a507 = 1u32;
                                    fixed_u32::bit_and(a506, a507)
                                };
                                let a509 = 0u32;
                                if equal(a508, a509) {
                                    Bytes::lit(&[0x1a])
                                } else {
                                    Bytes::lit(&[0x1b])
                                }
                            }
                        } else {
                            let a512 = {
                                let a510 = v0;
                                let a511 = 2u32;
                                fixed_u32::bit_and(a510, a511)
                            };
                            let a513 = 0u32;
                            if equal(a512, a513) {
                                let a516 = {
                                    let a514 = v0;
                                    let a515 = 1u32;
                                    fixed_u32::bit_and(a514, a515)
                                };
                                let a517 = 0u32;
                                if equal(a516, a517) {
                                    Bytes::lit(&[0x1c])
                                } else {
                                    Bytes::lit(&[0x1d])
                                }
                            } else {
                                let a520 = {
                                    let a518 = v0;
                                    let a519 = 1u32;
                                    fixed_u32::bit_and(a518, a519)
                                };
                                let a521 = 0u32;
                                if equal(a520, a521) {
                                    Bytes::lit(&[0x1e])
                                } else {
                                    Bytes::lit(&[0x1f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a524 = {
                    let a522 = v0;
                    let a523 = 16u32;
                    fixed_u32::bit_and(a522, a523)
                };
                let a525 = 0u32;
                if equal(a524, a525) {
                    let a528 = {
                        let a526 = v0;
                        let a527 = 8u32;
                        fixed_u32::bit_and(a526, a527)
                    };
                    let a529 = 0u32;
                    if equal(a528, a529) {
                        let a532 = {
                            let a530 = v0;
                            let a531 = 4u32;
                            fixed_u32::bit_and(a530, a531)
                        };
                        let a533 = 0u32;
                        if equal(a532, a533) {
                            let a536 = {
                                let a534 = v0;
                                let a535 = 2u32;
                                fixed_u32::bit_and(a534, a535)
                            };
                            let a537 = 0u32;
                            if equal(a536, a537) {
                                let a540 = {
                                    let a538 = v0;
                                    let a539 = 1u32;
                                    fixed_u32::bit_and(a538, a539)
                                };
                                let a541 = 0u32;
                                if equal(a540, a541) {
                                    Bytes::lit(&[0x20])
                                } else {
                                    Bytes::lit(&[0x21])
                                }
                            } else {
                                let a544 = {
                                    let a542 = v0;
                                    let a543 = 1u32;
                                    fixed_u32::bit_and(a542, a543)
                                };
                                let a545 = 0u32;
                                if equal(a544, a545) {
                                    Bytes::lit(&[0x22])
                                } else {
                                    Bytes::lit(&[0x23])
                                }
                            }
                        } else {
                            let a548 = {
                                let a546 = v0;
                                let a547 = 2u32;
                                fixed_u32::bit_and(a546, a547)
                            };
                            let a549 = 0u32;
                            if equal(a548, a549) {
                                let a552 = {
                                    let a550 = v0;
                                    let a551 = 1u32;
                                    fixed_u32::bit_and(a550, a551)
                                };
                                let a553 = 0u32;
                                if equal(a552, a553) {
                                    Bytes::lit(&[0x24])
                                } else {
                                    Bytes::lit(&[0x25])
                                }
                            } else {
                                let a556 = {
                                    let a554 = v0;
                                    let a555 = 1u32;
                                    fixed_u32::bit_and(a554, a555)
                                };
                                let a557 = 0u32;
                                if equal(a556, a557) {
                                    Bytes::lit(&[0x26])
                                } else {
                                    Bytes::lit(&[0x27])
                                }
                            }
                        }
                    } else {
                        let a560 = {
                            let a558 = v0;
                            let a559 = 4u32;
                            fixed_u32::bit_and(a558, a559)
                        };
                        let a561 = 0u32;
                        if equal(a560, a561) {
                            let a564 = {
                                let a562 = v0;
                                let a563 = 2u32;
                                fixed_u32::bit_and(a562, a563)
                            };
                            let a565 = 0u32;
                            if equal(a564, a565) {
                                let a568 = {
                                    let a566 = v0;
                                    let a567 = 1u32;
                                    fixed_u32::bit_and(a566, a567)
                                };
                                let a569 = 0u32;
                                if equal(a568, a569) {
                                    Bytes::lit(&[0x28])
                                } else {
                                    Bytes::lit(&[0x29])
                                }
                            } else {
                                let a572 = {
                                    let a570 = v0;
                                    let a571 = 1u32;
                                    fixed_u32::bit_and(a570, a571)
                                };
                                let a573 = 0u32;
                                if equal(a572, a573) {
                                    Bytes::lit(&[0x2a])
                                } else {
                                    Bytes::lit(&[0x2b])
                                }
                            }
                        } else {
                            let a576 = {
                                let a574 = v0;
                                let a575 = 2u32;
                                fixed_u32::bit_and(a574, a575)
                            };
                            let a577 = 0u32;
                            if equal(a576, a577) {
                                let a580 = {
                                    let a578 = v0;
                                    let a579 = 1u32;
                                    fixed_u32::bit_and(a578, a579)
                                };
                                let a581 = 0u32;
                                if equal(a580, a581) {
                                    Bytes::lit(&[0x2c])
                                } else {
                                    Bytes::lit(&[0x2d])
                                }
                            } else {
                                let a584 = {
                                    let a582 = v0;
                                    let a583 = 1u32;
                                    fixed_u32::bit_and(a582, a583)
                                };
                                let a585 = 0u32;
                                if equal(a584, a585) {
                                    Bytes::lit(&[0x2e])
                                } else {
                                    Bytes::lit(&[0x2f])
                                }
                            }
                        }
                    }
                } else {
                    let a588 = {
                        let a586 = v0;
                        let a587 = 8u32;
                        fixed_u32::bit_and(a586, a587)
                    };
                    let a589 = 0u32;
                    if equal(a588, a589) {
                        let a592 = {
                            let a590 = v0;
                            let a591 = 4u32;
                            fixed_u32::bit_and(a590, a591)
                        };
                        let a593 = 0u32;
                        if equal(a592, a593) {
                            let a596 = {
                                let a594 = v0;
                                let a595 = 2u32;
                                fixed_u32::bit_and(a594, a595)
                            };
                            let a597 = 0u32;
                            if equal(a596, a597) {
                                let a600 = {
                                    let a598 = v0;
                                    let a599 = 1u32;
                                    fixed_u32::bit_and(a598, a599)
                                };
                                let a601 = 0u32;
                                if equal(a600, a601) {
                                    Bytes::lit(&[0x30])
                                } else {
                                    Bytes::lit(&[0x31])
                                }
                            } else {
                                let a604 = {
                                    let a602 = v0;
                                    let a603 = 1u32;
                                    fixed_u32::bit_and(a602, a603)
                                };
                                let a605 = 0u32;
                                if equal(a604, a605) {
                                    Bytes::lit(&[0x32])
                                } else {
                                    Bytes::lit(&[0x33])
                                }
                            }
                        } else {
                            let a608 = {
                                let a606 = v0;
                                let a607 = 2u32;
                                fixed_u32::bit_and(a606, a607)
                            };
                            let a609 = 0u32;
                            if equal(a608, a609) {
                                let a612 = {
                                    let a610 = v0;
                                    let a611 = 1u32;
                                    fixed_u32::bit_and(a610, a611)
                                };
                                let a613 = 0u32;
                                if equal(a612, a613) {
                                    Bytes::lit(&[0x34])
                                } else {
                                    Bytes::lit(&[0x35])
                                }
                            } else {
                                let a616 = {
                                    let a614 = v0;
                                    let a615 = 1u32;
                                    fixed_u32::bit_and(a614, a615)
                                };
                                let a617 = 0u32;
                                if equal(a616, a617) {
                                    Bytes::lit(&[0x36])
                                } else {
                                    Bytes::lit(&[0x37])
                                }
                            }
                        }
                    } else {
                        let a620 = {
                            let a618 = v0;
                            let a619 = 4u32;
                            fixed_u32::bit_and(a618, a619)
                        };
                        let a621 = 0u32;
                        if equal(a620, a621) {
                            let a624 = {
                                let a622 = v0;
                                let a623 = 2u32;
                                fixed_u32::bit_and(a622, a623)
                            };
                            let a625 = 0u32;
                            if equal(a624, a625) {
                                let a628 = {
                                    let a626 = v0;
                                    let a627 = 1u32;
                                    fixed_u32::bit_and(a626, a627)
                                };
                                let a629 = 0u32;
                                if equal(a628, a629) {
                                    Bytes::lit(&[0x38])
                                } else {
                                    Bytes::lit(&[0x39])
                                }
                            } else {
                                let a632 = {
                                    let a630 = v0;
                                    let a631 = 1u32;
                                    fixed_u32::bit_and(a630, a631)
                                };
                                let a633 = 0u32;
                                if equal(a632, a633) {
                                    Bytes::lit(&[0x3a])
                                } else {
                                    Bytes::lit(&[0x3b])
                                }
                            }
                        } else {
                            let a636 = {
                                let a634 = v0;
                                let a635 = 2u32;
                                fixed_u32::bit_and(a634, a635)
                            };
                            let a637 = 0u32;
                            if equal(a636, a637) {
                                let a640 = {
                                    let a638 = v0;
                                    let a639 = 1u32;
                                    fixed_u32::bit_and(a638, a639)
                                };
                                let a641 = 0u32;
                                if equal(a640, a641) {
                                    Bytes::lit(&[0x3c])
                                } else {
                                    Bytes::lit(&[0x3d])
                                }
                            } else {
                                let a644 = {
                                    let a642 = v0;
                                    let a643 = 1u32;
                                    fixed_u32::bit_and(a642, a643)
                                };
                                let a645 = 0u32;
                                if equal(a644, a645) {
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
            let a648 = {
                let a646 = v0;
                let a647 = 32u32;
                fixed_u32::bit_and(a646, a647)
            };
            let a649 = 0u32;
            if equal(a648, a649) {
                let a652 = {
                    let a650 = v0;
                    let a651 = 16u32;
                    fixed_u32::bit_and(a650, a651)
                };
                let a653 = 0u32;
                if equal(a652, a653) {
                    let a656 = {
                        let a654 = v0;
                        let a655 = 8u32;
                        fixed_u32::bit_and(a654, a655)
                    };
                    let a657 = 0u32;
                    if equal(a656, a657) {
                        let a660 = {
                            let a658 = v0;
                            let a659 = 4u32;
                            fixed_u32::bit_and(a658, a659)
                        };
                        let a661 = 0u32;
                        if equal(a660, a661) {
                            let a664 = {
                                let a662 = v0;
                                let a663 = 2u32;
                                fixed_u32::bit_and(a662, a663)
                            };
                            let a665 = 0u32;
                            if equal(a664, a665) {
                                let a668 = {
                                    let a666 = v0;
                                    let a667 = 1u32;
                                    fixed_u32::bit_and(a666, a667)
                                };
                                let a669 = 0u32;
                                if equal(a668, a669) {
                                    Bytes::lit(&[0x40])
                                } else {
                                    Bytes::lit(&[0x41])
                                }
                            } else {
                                let a672 = {
                                    let a670 = v0;
                                    let a671 = 1u32;
                                    fixed_u32::bit_and(a670, a671)
                                };
                                let a673 = 0u32;
                                if equal(a672, a673) {
                                    Bytes::lit(&[0x42])
                                } else {
                                    Bytes::lit(&[0x43])
                                }
                            }
                        } else {
                            let a676 = {
                                let a674 = v0;
                                let a675 = 2u32;
                                fixed_u32::bit_and(a674, a675)
                            };
                            let a677 = 0u32;
                            if equal(a676, a677) {
                                let a680 = {
                                    let a678 = v0;
                                    let a679 = 1u32;
                                    fixed_u32::bit_and(a678, a679)
                                };
                                let a681 = 0u32;
                                if equal(a680, a681) {
                                    Bytes::lit(&[0x44])
                                } else {
                                    Bytes::lit(&[0x45])
                                }
                            } else {
                                let a684 = {
                                    let a682 = v0;
                                    let a683 = 1u32;
                                    fixed_u32::bit_and(a682, a683)
                                };
                                let a685 = 0u32;
                                if equal(a684, a685) {
                                    Bytes::lit(&[0x46])
                                } else {
                                    Bytes::lit(&[0x47])
                                }
                            }
                        }
                    } else {
                        let a688 = {
                            let a686 = v0;
                            let a687 = 4u32;
                            fixed_u32::bit_and(a686, a687)
                        };
                        let a689 = 0u32;
                        if equal(a688, a689) {
                            let a692 = {
                                let a690 = v0;
                                let a691 = 2u32;
                                fixed_u32::bit_and(a690, a691)
                            };
                            let a693 = 0u32;
                            if equal(a692, a693) {
                                let a696 = {
                                    let a694 = v0;
                                    let a695 = 1u32;
                                    fixed_u32::bit_and(a694, a695)
                                };
                                let a697 = 0u32;
                                if equal(a696, a697) {
                                    Bytes::lit(&[0x48])
                                } else {
                                    Bytes::lit(&[0x49])
                                }
                            } else {
                                let a700 = {
                                    let a698 = v0;
                                    let a699 = 1u32;
                                    fixed_u32::bit_and(a698, a699)
                                };
                                let a701 = 0u32;
                                if equal(a700, a701) {
                                    Bytes::lit(&[0x4a])
                                } else {
                                    Bytes::lit(&[0x4b])
                                }
                            }
                        } else {
                            let a704 = {
                                let a702 = v0;
                                let a703 = 2u32;
                                fixed_u32::bit_and(a702, a703)
                            };
                            let a705 = 0u32;
                            if equal(a704, a705) {
                                let a708 = {
                                    let a706 = v0;
                                    let a707 = 1u32;
                                    fixed_u32::bit_and(a706, a707)
                                };
                                let a709 = 0u32;
                                if equal(a708, a709) {
                                    Bytes::lit(&[0x4c])
                                } else {
                                    Bytes::lit(&[0x4d])
                                }
                            } else {
                                let a712 = {
                                    let a710 = v0;
                                    let a711 = 1u32;
                                    fixed_u32::bit_and(a710, a711)
                                };
                                let a713 = 0u32;
                                if equal(a712, a713) {
                                    Bytes::lit(&[0x4e])
                                } else {
                                    Bytes::lit(&[0x4f])
                                }
                            }
                        }
                    }
                } else {
                    let a716 = {
                        let a714 = v0;
                        let a715 = 8u32;
                        fixed_u32::bit_and(a714, a715)
                    };
                    let a717 = 0u32;
                    if equal(a716, a717) {
                        let a720 = {
                            let a718 = v0;
                            let a719 = 4u32;
                            fixed_u32::bit_and(a718, a719)
                        };
                        let a721 = 0u32;
                        if equal(a720, a721) {
                            let a724 = {
                                let a722 = v0;
                                let a723 = 2u32;
                                fixed_u32::bit_and(a722, a723)
                            };
                            let a725 = 0u32;
                            if equal(a724, a725) {
                                let a728 = {
                                    let a726 = v0;
                                    let a727 = 1u32;
                                    fixed_u32::bit_and(a726, a727)
                                };
                                let a729 = 0u32;
                                if equal(a728, a729) {
                                    Bytes::lit(&[0x50])
                                } else {
                                    Bytes::lit(&[0x51])
                                }
                            } else {
                                let a732 = {
                                    let a730 = v0;
                                    let a731 = 1u32;
                                    fixed_u32::bit_and(a730, a731)
                                };
                                let a733 = 0u32;
                                if equal(a732, a733) {
                                    Bytes::lit(&[0x52])
                                } else {
                                    Bytes::lit(&[0x53])
                                }
                            }
                        } else {
                            let a736 = {
                                let a734 = v0;
                                let a735 = 2u32;
                                fixed_u32::bit_and(a734, a735)
                            };
                            let a737 = 0u32;
                            if equal(a736, a737) {
                                let a740 = {
                                    let a738 = v0;
                                    let a739 = 1u32;
                                    fixed_u32::bit_and(a738, a739)
                                };
                                let a741 = 0u32;
                                if equal(a740, a741) {
                                    Bytes::lit(&[0x54])
                                } else {
                                    Bytes::lit(&[0x55])
                                }
                            } else {
                                let a744 = {
                                    let a742 = v0;
                                    let a743 = 1u32;
                                    fixed_u32::bit_and(a742, a743)
                                };
                                let a745 = 0u32;
                                if equal(a744, a745) {
                                    Bytes::lit(&[0x56])
                                } else {
                                    Bytes::lit(&[0x57])
                                }
                            }
                        }
                    } else {
                        let a748 = {
                            let a746 = v0;
                            let a747 = 4u32;
                            fixed_u32::bit_and(a746, a747)
                        };
                        let a749 = 0u32;
                        if equal(a748, a749) {
                            let a752 = {
                                let a750 = v0;
                                let a751 = 2u32;
                                fixed_u32::bit_and(a750, a751)
                            };
                            let a753 = 0u32;
                            if equal(a752, a753) {
                                let a756 = {
                                    let a754 = v0;
                                    let a755 = 1u32;
                                    fixed_u32::bit_and(a754, a755)
                                };
                                let a757 = 0u32;
                                if equal(a756, a757) {
                                    Bytes::lit(&[0x58])
                                } else {
                                    Bytes::lit(&[0x59])
                                }
                            } else {
                                let a760 = {
                                    let a758 = v0;
                                    let a759 = 1u32;
                                    fixed_u32::bit_and(a758, a759)
                                };
                                let a761 = 0u32;
                                if equal(a760, a761) {
                                    Bytes::lit(&[0x5a])
                                } else {
                                    Bytes::lit(&[0x5b])
                                }
                            }
                        } else {
                            let a764 = {
                                let a762 = v0;
                                let a763 = 2u32;
                                fixed_u32::bit_and(a762, a763)
                            };
                            let a765 = 0u32;
                            if equal(a764, a765) {
                                let a768 = {
                                    let a766 = v0;
                                    let a767 = 1u32;
                                    fixed_u32::bit_and(a766, a767)
                                };
                                let a769 = 0u32;
                                if equal(a768, a769) {
                                    Bytes::lit(&[0x5c])
                                } else {
                                    Bytes::lit(&[0x5d])
                                }
                            } else {
                                let a772 = {
                                    let a770 = v0;
                                    let a771 = 1u32;
                                    fixed_u32::bit_and(a770, a771)
                                };
                                let a773 = 0u32;
                                if equal(a772, a773) {
                                    Bytes::lit(&[0x5e])
                                } else {
                                    Bytes::lit(&[0x5f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a776 = {
                    let a774 = v0;
                    let a775 = 16u32;
                    fixed_u32::bit_and(a774, a775)
                };
                let a777 = 0u32;
                if equal(a776, a777) {
                    let a780 = {
                        let a778 = v0;
                        let a779 = 8u32;
                        fixed_u32::bit_and(a778, a779)
                    };
                    let a781 = 0u32;
                    if equal(a780, a781) {
                        let a784 = {
                            let a782 = v0;
                            let a783 = 4u32;
                            fixed_u32::bit_and(a782, a783)
                        };
                        let a785 = 0u32;
                        if equal(a784, a785) {
                            let a788 = {
                                let a786 = v0;
                                let a787 = 2u32;
                                fixed_u32::bit_and(a786, a787)
                            };
                            let a789 = 0u32;
                            if equal(a788, a789) {
                                let a792 = {
                                    let a790 = v0;
                                    let a791 = 1u32;
                                    fixed_u32::bit_and(a790, a791)
                                };
                                let a793 = 0u32;
                                if equal(a792, a793) {
                                    Bytes::lit(&[0x60])
                                } else {
                                    Bytes::lit(&[0x61])
                                }
                            } else {
                                let a796 = {
                                    let a794 = v0;
                                    let a795 = 1u32;
                                    fixed_u32::bit_and(a794, a795)
                                };
                                let a797 = 0u32;
                                if equal(a796, a797) {
                                    Bytes::lit(&[0x62])
                                } else {
                                    Bytes::lit(&[0x63])
                                }
                            }
                        } else {
                            let a800 = {
                                let a798 = v0;
                                let a799 = 2u32;
                                fixed_u32::bit_and(a798, a799)
                            };
                            let a801 = 0u32;
                            if equal(a800, a801) {
                                let a804 = {
                                    let a802 = v0;
                                    let a803 = 1u32;
                                    fixed_u32::bit_and(a802, a803)
                                };
                                let a805 = 0u32;
                                if equal(a804, a805) {
                                    Bytes::lit(&[0x64])
                                } else {
                                    Bytes::lit(&[0x65])
                                }
                            } else {
                                let a808 = {
                                    let a806 = v0;
                                    let a807 = 1u32;
                                    fixed_u32::bit_and(a806, a807)
                                };
                                let a809 = 0u32;
                                if equal(a808, a809) {
                                    Bytes::lit(&[0x66])
                                } else {
                                    Bytes::lit(&[0x67])
                                }
                            }
                        }
                    } else {
                        let a812 = {
                            let a810 = v0;
                            let a811 = 4u32;
                            fixed_u32::bit_and(a810, a811)
                        };
                        let a813 = 0u32;
                        if equal(a812, a813) {
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
                                    Bytes::lit(&[0x68])
                                } else {
                                    Bytes::lit(&[0x69])
                                }
                            } else {
                                let a824 = {
                                    let a822 = v0;
                                    let a823 = 1u32;
                                    fixed_u32::bit_and(a822, a823)
                                };
                                let a825 = 0u32;
                                if equal(a824, a825) {
                                    Bytes::lit(&[0x6a])
                                } else {
                                    Bytes::lit(&[0x6b])
                                }
                            }
                        } else {
                            let a828 = {
                                let a826 = v0;
                                let a827 = 2u32;
                                fixed_u32::bit_and(a826, a827)
                            };
                            let a829 = 0u32;
                            if equal(a828, a829) {
                                let a832 = {
                                    let a830 = v0;
                                    let a831 = 1u32;
                                    fixed_u32::bit_and(a830, a831)
                                };
                                let a833 = 0u32;
                                if equal(a832, a833) {
                                    Bytes::lit(&[0x6c])
                                } else {
                                    Bytes::lit(&[0x6d])
                                }
                            } else {
                                let a836 = {
                                    let a834 = v0;
                                    let a835 = 1u32;
                                    fixed_u32::bit_and(a834, a835)
                                };
                                let a837 = 0u32;
                                if equal(a836, a837) {
                                    Bytes::lit(&[0x6e])
                                } else {
                                    Bytes::lit(&[0x6f])
                                }
                            }
                        }
                    }
                } else {
                    let a840 = {
                        let a838 = v0;
                        let a839 = 8u32;
                        fixed_u32::bit_and(a838, a839)
                    };
                    let a841 = 0u32;
                    if equal(a840, a841) {
                        let a844 = {
                            let a842 = v0;
                            let a843 = 4u32;
                            fixed_u32::bit_and(a842, a843)
                        };
                        let a845 = 0u32;
                        if equal(a844, a845) {
                            let a848 = {
                                let a846 = v0;
                                let a847 = 2u32;
                                fixed_u32::bit_and(a846, a847)
                            };
                            let a849 = 0u32;
                            if equal(a848, a849) {
                                let a852 = {
                                    let a850 = v0;
                                    let a851 = 1u32;
                                    fixed_u32::bit_and(a850, a851)
                                };
                                let a853 = 0u32;
                                if equal(a852, a853) {
                                    Bytes::lit(&[0x70])
                                } else {
                                    Bytes::lit(&[0x71])
                                }
                            } else {
                                let a856 = {
                                    let a854 = v0;
                                    let a855 = 1u32;
                                    fixed_u32::bit_and(a854, a855)
                                };
                                let a857 = 0u32;
                                if equal(a856, a857) {
                                    Bytes::lit(&[0x72])
                                } else {
                                    Bytes::lit(&[0x73])
                                }
                            }
                        } else {
                            let a860 = {
                                let a858 = v0;
                                let a859 = 2u32;
                                fixed_u32::bit_and(a858, a859)
                            };
                            let a861 = 0u32;
                            if equal(a860, a861) {
                                let a864 = {
                                    let a862 = v0;
                                    let a863 = 1u32;
                                    fixed_u32::bit_and(a862, a863)
                                };
                                let a865 = 0u32;
                                if equal(a864, a865) {
                                    Bytes::lit(&[0x74])
                                } else {
                                    Bytes::lit(&[0x75])
                                }
                            } else {
                                let a868 = {
                                    let a866 = v0;
                                    let a867 = 1u32;
                                    fixed_u32::bit_and(a866, a867)
                                };
                                let a869 = 0u32;
                                if equal(a868, a869) {
                                    Bytes::lit(&[0x76])
                                } else {
                                    Bytes::lit(&[0x77])
                                }
                            }
                        }
                    } else {
                        let a872 = {
                            let a870 = v0;
                            let a871 = 4u32;
                            fixed_u32::bit_and(a870, a871)
                        };
                        let a873 = 0u32;
                        if equal(a872, a873) {
                            let a876 = {
                                let a874 = v0;
                                let a875 = 2u32;
                                fixed_u32::bit_and(a874, a875)
                            };
                            let a877 = 0u32;
                            if equal(a876, a877) {
                                let a880 = {
                                    let a878 = v0;
                                    let a879 = 1u32;
                                    fixed_u32::bit_and(a878, a879)
                                };
                                let a881 = 0u32;
                                if equal(a880, a881) {
                                    Bytes::lit(&[0x78])
                                } else {
                                    Bytes::lit(&[0x79])
                                }
                            } else {
                                let a884 = {
                                    let a882 = v0;
                                    let a883 = 1u32;
                                    fixed_u32::bit_and(a882, a883)
                                };
                                let a885 = 0u32;
                                if equal(a884, a885) {
                                    Bytes::lit(&[0x7a])
                                } else {
                                    Bytes::lit(&[0x7b])
                                }
                            }
                        } else {
                            let a888 = {
                                let a886 = v0;
                                let a887 = 2u32;
                                fixed_u32::bit_and(a886, a887)
                            };
                            let a889 = 0u32;
                            if equal(a888, a889) {
                                let a892 = {
                                    let a890 = v0;
                                    let a891 = 1u32;
                                    fixed_u32::bit_and(a890, a891)
                                };
                                let a893 = 0u32;
                                if equal(a892, a893) {
                                    Bytes::lit(&[0x7c])
                                } else {
                                    Bytes::lit(&[0x7d])
                                }
                            } else {
                                let a896 = {
                                    let a894 = v0;
                                    let a895 = 1u32;
                                    fixed_u32::bit_and(a894, a895)
                                };
                                let a897 = 0u32;
                                if equal(a896, a897) {
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
        let a900 = {
            let a898 = v0;
            let a899 = 64u32;
            fixed_u32::bit_and(a898, a899)
        };
        let a901 = 0u32;
        if equal(a900, a901) {
            let a904 = {
                let a902 = v0;
                let a903 = 32u32;
                fixed_u32::bit_and(a902, a903)
            };
            let a905 = 0u32;
            if equal(a904, a905) {
                let a908 = {
                    let a906 = v0;
                    let a907 = 16u32;
                    fixed_u32::bit_and(a906, a907)
                };
                let a909 = 0u32;
                if equal(a908, a909) {
                    let a912 = {
                        let a910 = v0;
                        let a911 = 8u32;
                        fixed_u32::bit_and(a910, a911)
                    };
                    let a913 = 0u32;
                    if equal(a912, a913) {
                        let a916 = {
                            let a914 = v0;
                            let a915 = 4u32;
                            fixed_u32::bit_and(a914, a915)
                        };
                        let a917 = 0u32;
                        if equal(a916, a917) {
                            let a920 = {
                                let a918 = v0;
                                let a919 = 2u32;
                                fixed_u32::bit_and(a918, a919)
                            };
                            let a921 = 0u32;
                            if equal(a920, a921) {
                                let a924 = {
                                    let a922 = v0;
                                    let a923 = 1u32;
                                    fixed_u32::bit_and(a922, a923)
                                };
                                let a925 = 0u32;
                                if equal(a924, a925) {
                                    Bytes::lit(&[0x80])
                                } else {
                                    Bytes::lit(&[0x81])
                                }
                            } else {
                                let a928 = {
                                    let a926 = v0;
                                    let a927 = 1u32;
                                    fixed_u32::bit_and(a926, a927)
                                };
                                let a929 = 0u32;
                                if equal(a928, a929) {
                                    Bytes::lit(&[0x82])
                                } else {
                                    Bytes::lit(&[0x83])
                                }
                            }
                        } else {
                            let a932 = {
                                let a930 = v0;
                                let a931 = 2u32;
                                fixed_u32::bit_and(a930, a931)
                            };
                            let a933 = 0u32;
                            if equal(a932, a933) {
                                let a936 = {
                                    let a934 = v0;
                                    let a935 = 1u32;
                                    fixed_u32::bit_and(a934, a935)
                                };
                                let a937 = 0u32;
                                if equal(a936, a937) {
                                    Bytes::lit(&[0x84])
                                } else {
                                    Bytes::lit(&[0x85])
                                }
                            } else {
                                let a940 = {
                                    let a938 = v0;
                                    let a939 = 1u32;
                                    fixed_u32::bit_and(a938, a939)
                                };
                                let a941 = 0u32;
                                if equal(a940, a941) {
                                    Bytes::lit(&[0x86])
                                } else {
                                    Bytes::lit(&[0x87])
                                }
                            }
                        }
                    } else {
                        let a944 = {
                            let a942 = v0;
                            let a943 = 4u32;
                            fixed_u32::bit_and(a942, a943)
                        };
                        let a945 = 0u32;
                        if equal(a944, a945) {
                            let a948 = {
                                let a946 = v0;
                                let a947 = 2u32;
                                fixed_u32::bit_and(a946, a947)
                            };
                            let a949 = 0u32;
                            if equal(a948, a949) {
                                let a952 = {
                                    let a950 = v0;
                                    let a951 = 1u32;
                                    fixed_u32::bit_and(a950, a951)
                                };
                                let a953 = 0u32;
                                if equal(a952, a953) {
                                    Bytes::lit(&[0x88])
                                } else {
                                    Bytes::lit(&[0x89])
                                }
                            } else {
                                let a956 = {
                                    let a954 = v0;
                                    let a955 = 1u32;
                                    fixed_u32::bit_and(a954, a955)
                                };
                                let a957 = 0u32;
                                if equal(a956, a957) {
                                    Bytes::lit(&[0x8a])
                                } else {
                                    Bytes::lit(&[0x8b])
                                }
                            }
                        } else {
                            let a960 = {
                                let a958 = v0;
                                let a959 = 2u32;
                                fixed_u32::bit_and(a958, a959)
                            };
                            let a961 = 0u32;
                            if equal(a960, a961) {
                                let a964 = {
                                    let a962 = v0;
                                    let a963 = 1u32;
                                    fixed_u32::bit_and(a962, a963)
                                };
                                let a965 = 0u32;
                                if equal(a964, a965) {
                                    Bytes::lit(&[0x8c])
                                } else {
                                    Bytes::lit(&[0x8d])
                                }
                            } else {
                                let a968 = {
                                    let a966 = v0;
                                    let a967 = 1u32;
                                    fixed_u32::bit_and(a966, a967)
                                };
                                let a969 = 0u32;
                                if equal(a968, a969) {
                                    Bytes::lit(&[0x8e])
                                } else {
                                    Bytes::lit(&[0x8f])
                                }
                            }
                        }
                    }
                } else {
                    let a972 = {
                        let a970 = v0;
                        let a971 = 8u32;
                        fixed_u32::bit_and(a970, a971)
                    };
                    let a973 = 0u32;
                    if equal(a972, a973) {
                        let a976 = {
                            let a974 = v0;
                            let a975 = 4u32;
                            fixed_u32::bit_and(a974, a975)
                        };
                        let a977 = 0u32;
                        if equal(a976, a977) {
                            let a980 = {
                                let a978 = v0;
                                let a979 = 2u32;
                                fixed_u32::bit_and(a978, a979)
                            };
                            let a981 = 0u32;
                            if equal(a980, a981) {
                                let a984 = {
                                    let a982 = v0;
                                    let a983 = 1u32;
                                    fixed_u32::bit_and(a982, a983)
                                };
                                let a985 = 0u32;
                                if equal(a984, a985) {
                                    Bytes::lit(&[0x90])
                                } else {
                                    Bytes::lit(&[0x91])
                                }
                            } else {
                                let a988 = {
                                    let a986 = v0;
                                    let a987 = 1u32;
                                    fixed_u32::bit_and(a986, a987)
                                };
                                let a989 = 0u32;
                                if equal(a988, a989) {
                                    Bytes::lit(&[0x92])
                                } else {
                                    Bytes::lit(&[0x93])
                                }
                            }
                        } else {
                            let a992 = {
                                let a990 = v0;
                                let a991 = 2u32;
                                fixed_u32::bit_and(a990, a991)
                            };
                            let a993 = 0u32;
                            if equal(a992, a993) {
                                let a996 = {
                                    let a994 = v0;
                                    let a995 = 1u32;
                                    fixed_u32::bit_and(a994, a995)
                                };
                                let a997 = 0u32;
                                if equal(a996, a997) {
                                    Bytes::lit(&[0x94])
                                } else {
                                    Bytes::lit(&[0x95])
                                }
                            } else {
                                let a1000 = {
                                    let a998 = v0;
                                    let a999 = 1u32;
                                    fixed_u32::bit_and(a998, a999)
                                };
                                let a1001 = 0u32;
                                if equal(a1000, a1001) {
                                    Bytes::lit(&[0x96])
                                } else {
                                    Bytes::lit(&[0x97])
                                }
                            }
                        }
                    } else {
                        let a1004 = {
                            let a1002 = v0;
                            let a1003 = 4u32;
                            fixed_u32::bit_and(a1002, a1003)
                        };
                        let a1005 = 0u32;
                        if equal(a1004, a1005) {
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
                                    Bytes::lit(&[0x98])
                                } else {
                                    Bytes::lit(&[0x99])
                                }
                            } else {
                                let a1016 = {
                                    let a1014 = v0;
                                    let a1015 = 1u32;
                                    fixed_u32::bit_and(a1014, a1015)
                                };
                                let a1017 = 0u32;
                                if equal(a1016, a1017) {
                                    Bytes::lit(&[0x9a])
                                } else {
                                    Bytes::lit(&[0x9b])
                                }
                            }
                        } else {
                            let a1020 = {
                                let a1018 = v0;
                                let a1019 = 2u32;
                                fixed_u32::bit_and(a1018, a1019)
                            };
                            let a1021 = 0u32;
                            if equal(a1020, a1021) {
                                let a1024 = {
                                    let a1022 = v0;
                                    let a1023 = 1u32;
                                    fixed_u32::bit_and(a1022, a1023)
                                };
                                let a1025 = 0u32;
                                if equal(a1024, a1025) {
                                    Bytes::lit(&[0x9c])
                                } else {
                                    Bytes::lit(&[0x9d])
                                }
                            } else {
                                let a1028 = {
                                    let a1026 = v0;
                                    let a1027 = 1u32;
                                    fixed_u32::bit_and(a1026, a1027)
                                };
                                let a1029 = 0u32;
                                if equal(a1028, a1029) {
                                    Bytes::lit(&[0x9e])
                                } else {
                                    Bytes::lit(&[0x9f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a1032 = {
                    let a1030 = v0;
                    let a1031 = 16u32;
                    fixed_u32::bit_and(a1030, a1031)
                };
                let a1033 = 0u32;
                if equal(a1032, a1033) {
                    let a1036 = {
                        let a1034 = v0;
                        let a1035 = 8u32;
                        fixed_u32::bit_and(a1034, a1035)
                    };
                    let a1037 = 0u32;
                    if equal(a1036, a1037) {
                        let a1040 = {
                            let a1038 = v0;
                            let a1039 = 4u32;
                            fixed_u32::bit_and(a1038, a1039)
                        };
                        let a1041 = 0u32;
                        if equal(a1040, a1041) {
                            let a1044 = {
                                let a1042 = v0;
                                let a1043 = 2u32;
                                fixed_u32::bit_and(a1042, a1043)
                            };
                            let a1045 = 0u32;
                            if equal(a1044, a1045) {
                                let a1048 = {
                                    let a1046 = v0;
                                    let a1047 = 1u32;
                                    fixed_u32::bit_and(a1046, a1047)
                                };
                                let a1049 = 0u32;
                                if equal(a1048, a1049) {
                                    Bytes::lit(&[0xa0])
                                } else {
                                    Bytes::lit(&[0xa1])
                                }
                            } else {
                                let a1052 = {
                                    let a1050 = v0;
                                    let a1051 = 1u32;
                                    fixed_u32::bit_and(a1050, a1051)
                                };
                                let a1053 = 0u32;
                                if equal(a1052, a1053) {
                                    Bytes::lit(&[0xa2])
                                } else {
                                    Bytes::lit(&[0xa3])
                                }
                            }
                        } else {
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
                                    Bytes::lit(&[0xa4])
                                } else {
                                    Bytes::lit(&[0xa5])
                                }
                            } else {
                                let a1064 = {
                                    let a1062 = v0;
                                    let a1063 = 1u32;
                                    fixed_u32::bit_and(a1062, a1063)
                                };
                                let a1065 = 0u32;
                                if equal(a1064, a1065) {
                                    Bytes::lit(&[0xa6])
                                } else {
                                    Bytes::lit(&[0xa7])
                                }
                            }
                        }
                    } else {
                        let a1068 = {
                            let a1066 = v0;
                            let a1067 = 4u32;
                            fixed_u32::bit_and(a1066, a1067)
                        };
                        let a1069 = 0u32;
                        if equal(a1068, a1069) {
                            let a1072 = {
                                let a1070 = v0;
                                let a1071 = 2u32;
                                fixed_u32::bit_and(a1070, a1071)
                            };
                            let a1073 = 0u32;
                            if equal(a1072, a1073) {
                                let a1076 = {
                                    let a1074 = v0;
                                    let a1075 = 1u32;
                                    fixed_u32::bit_and(a1074, a1075)
                                };
                                let a1077 = 0u32;
                                if equal(a1076, a1077) {
                                    Bytes::lit(&[0xa8])
                                } else {
                                    Bytes::lit(&[0xa9])
                                }
                            } else {
                                let a1080 = {
                                    let a1078 = v0;
                                    let a1079 = 1u32;
                                    fixed_u32::bit_and(a1078, a1079)
                                };
                                let a1081 = 0u32;
                                if equal(a1080, a1081) {
                                    Bytes::lit(&[0xaa])
                                } else {
                                    Bytes::lit(&[0xab])
                                }
                            }
                        } else {
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
                                    Bytes::lit(&[0xac])
                                } else {
                                    Bytes::lit(&[0xad])
                                }
                            } else {
                                let a1092 = {
                                    let a1090 = v0;
                                    let a1091 = 1u32;
                                    fixed_u32::bit_and(a1090, a1091)
                                };
                                let a1093 = 0u32;
                                if equal(a1092, a1093) {
                                    Bytes::lit(&[0xae])
                                } else {
                                    Bytes::lit(&[0xaf])
                                }
                            }
                        }
                    }
                } else {
                    let a1096 = {
                        let a1094 = v0;
                        let a1095 = 8u32;
                        fixed_u32::bit_and(a1094, a1095)
                    };
                    let a1097 = 0u32;
                    if equal(a1096, a1097) {
                        let a1100 = {
                            let a1098 = v0;
                            let a1099 = 4u32;
                            fixed_u32::bit_and(a1098, a1099)
                        };
                        let a1101 = 0u32;
                        if equal(a1100, a1101) {
                            let a1104 = {
                                let a1102 = v0;
                                let a1103 = 2u32;
                                fixed_u32::bit_and(a1102, a1103)
                            };
                            let a1105 = 0u32;
                            if equal(a1104, a1105) {
                                let a1108 = {
                                    let a1106 = v0;
                                    let a1107 = 1u32;
                                    fixed_u32::bit_and(a1106, a1107)
                                };
                                let a1109 = 0u32;
                                if equal(a1108, a1109) {
                                    Bytes::lit(&[0xb0])
                                } else {
                                    Bytes::lit(&[0xb1])
                                }
                            } else {
                                let a1112 = {
                                    let a1110 = v0;
                                    let a1111 = 1u32;
                                    fixed_u32::bit_and(a1110, a1111)
                                };
                                let a1113 = 0u32;
                                if equal(a1112, a1113) {
                                    Bytes::lit(&[0xb2])
                                } else {
                                    Bytes::lit(&[0xb3])
                                }
                            }
                        } else {
                            let a1116 = {
                                let a1114 = v0;
                                let a1115 = 2u32;
                                fixed_u32::bit_and(a1114, a1115)
                            };
                            let a1117 = 0u32;
                            if equal(a1116, a1117) {
                                let a1120 = {
                                    let a1118 = v0;
                                    let a1119 = 1u32;
                                    fixed_u32::bit_and(a1118, a1119)
                                };
                                let a1121 = 0u32;
                                if equal(a1120, a1121) {
                                    Bytes::lit(&[0xb4])
                                } else {
                                    Bytes::lit(&[0xb5])
                                }
                            } else {
                                let a1124 = {
                                    let a1122 = v0;
                                    let a1123 = 1u32;
                                    fixed_u32::bit_and(a1122, a1123)
                                };
                                let a1125 = 0u32;
                                if equal(a1124, a1125) {
                                    Bytes::lit(&[0xb6])
                                } else {
                                    Bytes::lit(&[0xb7])
                                }
                            }
                        }
                    } else {
                        let a1128 = {
                            let a1126 = v0;
                            let a1127 = 4u32;
                            fixed_u32::bit_and(a1126, a1127)
                        };
                        let a1129 = 0u32;
                        if equal(a1128, a1129) {
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
                                    Bytes::lit(&[0xb8])
                                } else {
                                    Bytes::lit(&[0xb9])
                                }
                            } else {
                                let a1140 = {
                                    let a1138 = v0;
                                    let a1139 = 1u32;
                                    fixed_u32::bit_and(a1138, a1139)
                                };
                                let a1141 = 0u32;
                                if equal(a1140, a1141) {
                                    Bytes::lit(&[0xba])
                                } else {
                                    Bytes::lit(&[0xbb])
                                }
                            }
                        } else {
                            let a1144 = {
                                let a1142 = v0;
                                let a1143 = 2u32;
                                fixed_u32::bit_and(a1142, a1143)
                            };
                            let a1145 = 0u32;
                            if equal(a1144, a1145) {
                                let a1148 = {
                                    let a1146 = v0;
                                    let a1147 = 1u32;
                                    fixed_u32::bit_and(a1146, a1147)
                                };
                                let a1149 = 0u32;
                                if equal(a1148, a1149) {
                                    Bytes::lit(&[0xbc])
                                } else {
                                    Bytes::lit(&[0xbd])
                                }
                            } else {
                                let a1152 = {
                                    let a1150 = v0;
                                    let a1151 = 1u32;
                                    fixed_u32::bit_and(a1150, a1151)
                                };
                                let a1153 = 0u32;
                                if equal(a1152, a1153) {
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
            let a1156 = {
                let a1154 = v0;
                let a1155 = 32u32;
                fixed_u32::bit_and(a1154, a1155)
            };
            let a1157 = 0u32;
            if equal(a1156, a1157) {
                let a1160 = {
                    let a1158 = v0;
                    let a1159 = 16u32;
                    fixed_u32::bit_and(a1158, a1159)
                };
                let a1161 = 0u32;
                if equal(a1160, a1161) {
                    let a1164 = {
                        let a1162 = v0;
                        let a1163 = 8u32;
                        fixed_u32::bit_and(a1162, a1163)
                    };
                    let a1165 = 0u32;
                    if equal(a1164, a1165) {
                        let a1168 = {
                            let a1166 = v0;
                            let a1167 = 4u32;
                            fixed_u32::bit_and(a1166, a1167)
                        };
                        let a1169 = 0u32;
                        if equal(a1168, a1169) {
                            let a1172 = {
                                let a1170 = v0;
                                let a1171 = 2u32;
                                fixed_u32::bit_and(a1170, a1171)
                            };
                            let a1173 = 0u32;
                            if equal(a1172, a1173) {
                                let a1176 = {
                                    let a1174 = v0;
                                    let a1175 = 1u32;
                                    fixed_u32::bit_and(a1174, a1175)
                                };
                                let a1177 = 0u32;
                                if equal(a1176, a1177) {
                                    Bytes::lit(&[0xc0])
                                } else {
                                    Bytes::lit(&[0xc1])
                                }
                            } else {
                                let a1180 = {
                                    let a1178 = v0;
                                    let a1179 = 1u32;
                                    fixed_u32::bit_and(a1178, a1179)
                                };
                                let a1181 = 0u32;
                                if equal(a1180, a1181) {
                                    Bytes::lit(&[0xc2])
                                } else {
                                    Bytes::lit(&[0xc3])
                                }
                            }
                        } else {
                            let a1184 = {
                                let a1182 = v0;
                                let a1183 = 2u32;
                                fixed_u32::bit_and(a1182, a1183)
                            };
                            let a1185 = 0u32;
                            if equal(a1184, a1185) {
                                let a1188 = {
                                    let a1186 = v0;
                                    let a1187 = 1u32;
                                    fixed_u32::bit_and(a1186, a1187)
                                };
                                let a1189 = 0u32;
                                if equal(a1188, a1189) {
                                    Bytes::lit(&[0xc4])
                                } else {
                                    Bytes::lit(&[0xc5])
                                }
                            } else {
                                let a1192 = {
                                    let a1190 = v0;
                                    let a1191 = 1u32;
                                    fixed_u32::bit_and(a1190, a1191)
                                };
                                let a1193 = 0u32;
                                if equal(a1192, a1193) {
                                    Bytes::lit(&[0xc6])
                                } else {
                                    Bytes::lit(&[0xc7])
                                }
                            }
                        }
                    } else {
                        let a1196 = {
                            let a1194 = v0;
                            let a1195 = 4u32;
                            fixed_u32::bit_and(a1194, a1195)
                        };
                        let a1197 = 0u32;
                        if equal(a1196, a1197) {
                            let a1200 = {
                                let a1198 = v0;
                                let a1199 = 2u32;
                                fixed_u32::bit_and(a1198, a1199)
                            };
                            let a1201 = 0u32;
                            if equal(a1200, a1201) {
                                let a1204 = {
                                    let a1202 = v0;
                                    let a1203 = 1u32;
                                    fixed_u32::bit_and(a1202, a1203)
                                };
                                let a1205 = 0u32;
                                if equal(a1204, a1205) {
                                    Bytes::lit(&[0xc8])
                                } else {
                                    Bytes::lit(&[0xc9])
                                }
                            } else {
                                let a1208 = {
                                    let a1206 = v0;
                                    let a1207 = 1u32;
                                    fixed_u32::bit_and(a1206, a1207)
                                };
                                let a1209 = 0u32;
                                if equal(a1208, a1209) {
                                    Bytes::lit(&[0xca])
                                } else {
                                    Bytes::lit(&[0xcb])
                                }
                            }
                        } else {
                            let a1212 = {
                                let a1210 = v0;
                                let a1211 = 2u32;
                                fixed_u32::bit_and(a1210, a1211)
                            };
                            let a1213 = 0u32;
                            if equal(a1212, a1213) {
                                let a1216 = {
                                    let a1214 = v0;
                                    let a1215 = 1u32;
                                    fixed_u32::bit_and(a1214, a1215)
                                };
                                let a1217 = 0u32;
                                if equal(a1216, a1217) {
                                    Bytes::lit(&[0xcc])
                                } else {
                                    Bytes::lit(&[0xcd])
                                }
                            } else {
                                let a1220 = {
                                    let a1218 = v0;
                                    let a1219 = 1u32;
                                    fixed_u32::bit_and(a1218, a1219)
                                };
                                let a1221 = 0u32;
                                if equal(a1220, a1221) {
                                    Bytes::lit(&[0xce])
                                } else {
                                    Bytes::lit(&[0xcf])
                                }
                            }
                        }
                    }
                } else {
                    let a1224 = {
                        let a1222 = v0;
                        let a1223 = 8u32;
                        fixed_u32::bit_and(a1222, a1223)
                    };
                    let a1225 = 0u32;
                    if equal(a1224, a1225) {
                        let a1228 = {
                            let a1226 = v0;
                            let a1227 = 4u32;
                            fixed_u32::bit_and(a1226, a1227)
                        };
                        let a1229 = 0u32;
                        if equal(a1228, a1229) {
                            let a1232 = {
                                let a1230 = v0;
                                let a1231 = 2u32;
                                fixed_u32::bit_and(a1230, a1231)
                            };
                            let a1233 = 0u32;
                            if equal(a1232, a1233) {
                                let a1236 = {
                                    let a1234 = v0;
                                    let a1235 = 1u32;
                                    fixed_u32::bit_and(a1234, a1235)
                                };
                                let a1237 = 0u32;
                                if equal(a1236, a1237) {
                                    Bytes::lit(&[0xd0])
                                } else {
                                    Bytes::lit(&[0xd1])
                                }
                            } else {
                                let a1240 = {
                                    let a1238 = v0;
                                    let a1239 = 1u32;
                                    fixed_u32::bit_and(a1238, a1239)
                                };
                                let a1241 = 0u32;
                                if equal(a1240, a1241) {
                                    Bytes::lit(&[0xd2])
                                } else {
                                    Bytes::lit(&[0xd3])
                                }
                            }
                        } else {
                            let a1244 = {
                                let a1242 = v0;
                                let a1243 = 2u32;
                                fixed_u32::bit_and(a1242, a1243)
                            };
                            let a1245 = 0u32;
                            if equal(a1244, a1245) {
                                let a1248 = {
                                    let a1246 = v0;
                                    let a1247 = 1u32;
                                    fixed_u32::bit_and(a1246, a1247)
                                };
                                let a1249 = 0u32;
                                if equal(a1248, a1249) {
                                    Bytes::lit(&[0xd4])
                                } else {
                                    Bytes::lit(&[0xd5])
                                }
                            } else {
                                let a1252 = {
                                    let a1250 = v0;
                                    let a1251 = 1u32;
                                    fixed_u32::bit_and(a1250, a1251)
                                };
                                let a1253 = 0u32;
                                if equal(a1252, a1253) {
                                    Bytes::lit(&[0xd6])
                                } else {
                                    Bytes::lit(&[0xd7])
                                }
                            }
                        }
                    } else {
                        let a1256 = {
                            let a1254 = v0;
                            let a1255 = 4u32;
                            fixed_u32::bit_and(a1254, a1255)
                        };
                        let a1257 = 0u32;
                        if equal(a1256, a1257) {
                            let a1260 = {
                                let a1258 = v0;
                                let a1259 = 2u32;
                                fixed_u32::bit_and(a1258, a1259)
                            };
                            let a1261 = 0u32;
                            if equal(a1260, a1261) {
                                let a1264 = {
                                    let a1262 = v0;
                                    let a1263 = 1u32;
                                    fixed_u32::bit_and(a1262, a1263)
                                };
                                let a1265 = 0u32;
                                if equal(a1264, a1265) {
                                    Bytes::lit(&[0xd8])
                                } else {
                                    Bytes::lit(&[0xd9])
                                }
                            } else {
                                let a1268 = {
                                    let a1266 = v0;
                                    let a1267 = 1u32;
                                    fixed_u32::bit_and(a1266, a1267)
                                };
                                let a1269 = 0u32;
                                if equal(a1268, a1269) {
                                    Bytes::lit(&[0xda])
                                } else {
                                    Bytes::lit(&[0xdb])
                                }
                            }
                        } else {
                            let a1272 = {
                                let a1270 = v0;
                                let a1271 = 2u32;
                                fixed_u32::bit_and(a1270, a1271)
                            };
                            let a1273 = 0u32;
                            if equal(a1272, a1273) {
                                let a1276 = {
                                    let a1274 = v0;
                                    let a1275 = 1u32;
                                    fixed_u32::bit_and(a1274, a1275)
                                };
                                let a1277 = 0u32;
                                if equal(a1276, a1277) {
                                    Bytes::lit(&[0xdc])
                                } else {
                                    Bytes::lit(&[0xdd])
                                }
                            } else {
                                let a1280 = {
                                    let a1278 = v0;
                                    let a1279 = 1u32;
                                    fixed_u32::bit_and(a1278, a1279)
                                };
                                let a1281 = 0u32;
                                if equal(a1280, a1281) {
                                    Bytes::lit(&[0xde])
                                } else {
                                    Bytes::lit(&[0xdf])
                                }
                            }
                        }
                    }
                }
            } else {
                let a1284 = {
                    let a1282 = v0;
                    let a1283 = 16u32;
                    fixed_u32::bit_and(a1282, a1283)
                };
                let a1285 = 0u32;
                if equal(a1284, a1285) {
                    let a1288 = {
                        let a1286 = v0;
                        let a1287 = 8u32;
                        fixed_u32::bit_and(a1286, a1287)
                    };
                    let a1289 = 0u32;
                    if equal(a1288, a1289) {
                        let a1292 = {
                            let a1290 = v0;
                            let a1291 = 4u32;
                            fixed_u32::bit_and(a1290, a1291)
                        };
                        let a1293 = 0u32;
                        if equal(a1292, a1293) {
                            let a1296 = {
                                let a1294 = v0;
                                let a1295 = 2u32;
                                fixed_u32::bit_and(a1294, a1295)
                            };
                            let a1297 = 0u32;
                            if equal(a1296, a1297) {
                                let a1300 = {
                                    let a1298 = v0;
                                    let a1299 = 1u32;
                                    fixed_u32::bit_and(a1298, a1299)
                                };
                                let a1301 = 0u32;
                                if equal(a1300, a1301) {
                                    Bytes::lit(&[0xe0])
                                } else {
                                    Bytes::lit(&[0xe1])
                                }
                            } else {
                                let a1304 = {
                                    let a1302 = v0;
                                    let a1303 = 1u32;
                                    fixed_u32::bit_and(a1302, a1303)
                                };
                                let a1305 = 0u32;
                                if equal(a1304, a1305) {
                                    Bytes::lit(&[0xe2])
                                } else {
                                    Bytes::lit(&[0xe3])
                                }
                            }
                        } else {
                            let a1308 = {
                                let a1306 = v0;
                                let a1307 = 2u32;
                                fixed_u32::bit_and(a1306, a1307)
                            };
                            let a1309 = 0u32;
                            if equal(a1308, a1309) {
                                let a1312 = {
                                    let a1310 = v0;
                                    let a1311 = 1u32;
                                    fixed_u32::bit_and(a1310, a1311)
                                };
                                let a1313 = 0u32;
                                if equal(a1312, a1313) {
                                    Bytes::lit(&[0xe4])
                                } else {
                                    Bytes::lit(&[0xe5])
                                }
                            } else {
                                let a1316 = {
                                    let a1314 = v0;
                                    let a1315 = 1u32;
                                    fixed_u32::bit_and(a1314, a1315)
                                };
                                let a1317 = 0u32;
                                if equal(a1316, a1317) {
                                    Bytes::lit(&[0xe6])
                                } else {
                                    Bytes::lit(&[0xe7])
                                }
                            }
                        }
                    } else {
                        let a1320 = {
                            let a1318 = v0;
                            let a1319 = 4u32;
                            fixed_u32::bit_and(a1318, a1319)
                        };
                        let a1321 = 0u32;
                        if equal(a1320, a1321) {
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
                                    Bytes::lit(&[0xe8])
                                } else {
                                    Bytes::lit(&[0xe9])
                                }
                            } else {
                                let a1332 = {
                                    let a1330 = v0;
                                    let a1331 = 1u32;
                                    fixed_u32::bit_and(a1330, a1331)
                                };
                                let a1333 = 0u32;
                                if equal(a1332, a1333) {
                                    Bytes::lit(&[0xea])
                                } else {
                                    Bytes::lit(&[0xeb])
                                }
                            }
                        } else {
                            let a1336 = {
                                let a1334 = v0;
                                let a1335 = 2u32;
                                fixed_u32::bit_and(a1334, a1335)
                            };
                            let a1337 = 0u32;
                            if equal(a1336, a1337) {
                                let a1340 = {
                                    let a1338 = v0;
                                    let a1339 = 1u32;
                                    fixed_u32::bit_and(a1338, a1339)
                                };
                                let a1341 = 0u32;
                                if equal(a1340, a1341) {
                                    Bytes::lit(&[0xec])
                                } else {
                                    Bytes::lit(&[0xed])
                                }
                            } else {
                                let a1344 = {
                                    let a1342 = v0;
                                    let a1343 = 1u32;
                                    fixed_u32::bit_and(a1342, a1343)
                                };
                                let a1345 = 0u32;
                                if equal(a1344, a1345) {
                                    Bytes::lit(&[0xee])
                                } else {
                                    Bytes::lit(&[0xef])
                                }
                            }
                        }
                    }
                } else {
                    let a1348 = {
                        let a1346 = v0;
                        let a1347 = 8u32;
                        fixed_u32::bit_and(a1346, a1347)
                    };
                    let a1349 = 0u32;
                    if equal(a1348, a1349) {
                        let a1352 = {
                            let a1350 = v0;
                            let a1351 = 4u32;
                            fixed_u32::bit_and(a1350, a1351)
                        };
                        let a1353 = 0u32;
                        if equal(a1352, a1353) {
                            let a1356 = {
                                let a1354 = v0;
                                let a1355 = 2u32;
                                fixed_u32::bit_and(a1354, a1355)
                            };
                            let a1357 = 0u32;
                            if equal(a1356, a1357) {
                                let a1360 = {
                                    let a1358 = v0;
                                    let a1359 = 1u32;
                                    fixed_u32::bit_and(a1358, a1359)
                                };
                                let a1361 = 0u32;
                                if equal(a1360, a1361) {
                                    Bytes::lit(&[0xf0])
                                } else {
                                    Bytes::lit(&[0xf1])
                                }
                            } else {
                                let a1364 = {
                                    let a1362 = v0;
                                    let a1363 = 1u32;
                                    fixed_u32::bit_and(a1362, a1363)
                                };
                                let a1365 = 0u32;
                                if equal(a1364, a1365) {
                                    Bytes::lit(&[0xf2])
                                } else {
                                    Bytes::lit(&[0xf3])
                                }
                            }
                        } else {
                            let a1368 = {
                                let a1366 = v0;
                                let a1367 = 2u32;
                                fixed_u32::bit_and(a1366, a1367)
                            };
                            let a1369 = 0u32;
                            if equal(a1368, a1369) {
                                let a1372 = {
                                    let a1370 = v0;
                                    let a1371 = 1u32;
                                    fixed_u32::bit_and(a1370, a1371)
                                };
                                let a1373 = 0u32;
                                if equal(a1372, a1373) {
                                    Bytes::lit(&[0xf4])
                                } else {
                                    Bytes::lit(&[0xf5])
                                }
                            } else {
                                let a1376 = {
                                    let a1374 = v0;
                                    let a1375 = 1u32;
                                    fixed_u32::bit_and(a1374, a1375)
                                };
                                let a1377 = 0u32;
                                if equal(a1376, a1377) {
                                    Bytes::lit(&[0xf6])
                                } else {
                                    Bytes::lit(&[0xf7])
                                }
                            }
                        }
                    } else {
                        let a1380 = {
                            let a1378 = v0;
                            let a1379 = 4u32;
                            fixed_u32::bit_and(a1378, a1379)
                        };
                        let a1381 = 0u32;
                        if equal(a1380, a1381) {
                            let a1384 = {
                                let a1382 = v0;
                                let a1383 = 2u32;
                                fixed_u32::bit_and(a1382, a1383)
                            };
                            let a1385 = 0u32;
                            if equal(a1384, a1385) {
                                let a1388 = {
                                    let a1386 = v0;
                                    let a1387 = 1u32;
                                    fixed_u32::bit_and(a1386, a1387)
                                };
                                let a1389 = 0u32;
                                if equal(a1388, a1389) {
                                    Bytes::lit(&[0xf8])
                                } else {
                                    Bytes::lit(&[0xf9])
                                }
                            } else {
                                let a1392 = {
                                    let a1390 = v0;
                                    let a1391 = 1u32;
                                    fixed_u32::bit_and(a1390, a1391)
                                };
                                let a1393 = 0u32;
                                if equal(a1392, a1393) {
                                    Bytes::lit(&[0xfa])
                                } else {
                                    Bytes::lit(&[0xfb])
                                }
                            }
                        } else {
                            let a1396 = {
                                let a1394 = v0;
                                let a1395 = 2u32;
                                fixed_u32::bit_and(a1394, a1395)
                            };
                            let a1397 = 0u32;
                            if equal(a1396, a1397) {
                                let a1400 = {
                                    let a1398 = v0;
                                    let a1399 = 1u32;
                                    fixed_u32::bit_and(a1398, a1399)
                                };
                                let a1401 = 0u32;
                                if equal(a1400, a1401) {
                                    Bytes::lit(&[0xfc])
                                } else {
                                    Bytes::lit(&[0xfd])
                                }
                            } else {
                                let a1404 = {
                                    let a1402 = v0;
                                    let a1403 = 1u32;
                                    fixed_u32::bit_and(a1402, a1403)
                                };
                                let a1405 = 0u32;
                                if equal(a1404, a1405) {
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

pub fn f56(v0: u32, v1: u32) -> u32 {
    let m1408 = {
        let a1406 = v0;
        let a1407 = v1;
        fixed_u32::shift_right(a1406, a1407)
    };
    match m1408 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f57(v0: Adt3, v1: u32, v2: (Adt3, (u64, List<Adt3>))) -> Option<(Adt3, (u64, List<Adt3>))> {
    let v3: u64 = {
        let (h1409, _) = {
            let (_, h1410) = v2.clone();
            h1410
        };
        h1409
    };
    let a1414 = {
        let a1411 = v3;
        let a1412 = 2u64;
        let a1413 = 0u64;
        nat_rem(a1411, a1412, a1413)
    };
    let a1415 = 0u64;
    if nat_eq(a1414, a1415) {
        let m1418 = {
            let (_, h1416) = {
                let (_, h1417) = v2;
                h1417
            };
            h1416
        };
        match m1418.uncons() {
            None => {
                None::<(Adt3, (u64, List<Adt3>))>
            }
            Some((v4, v5)) => {
                Some((f32(f31(v4, {
                    let (h1419, _) = v2;
                    h1419
                }, v0, v1)), ({
                    let a1420 = v3;
                    let a1421 = 2u64;
                    let a1422 = 0u64;
                    nat_quot(a1420, a1421, a1422)
                }, v5)))
            }
        }
    } else {
        None::<(Adt3, (u64, List<Adt3>))>
    }
}

pub fn f58(v0: Adt5, v1: Adt5) -> Adt5 {
    let v2: Adt6 = f66(match v0.clone() {
        Adt5::C0(_, _, _, h1423, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1423
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, h1424, _, _, _, _, _, _, _, _, _, _, _) => {
            h1424
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, h1425, _, _, _, _, _, _) => {
            h1425
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1426, _) => {
            h1426
        }
    }, match v1.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1427, _) => {
            h1427
        }
    }, match v1 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1428) => {
            h1428
        }
    });
    Adt5::C0(match v0.clone() {
        Adt5::C0(h1429, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1429
        }
    }, match v0.clone() {
        Adt5::C0(_, h1430, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1430
        }
    }, match v0.clone() {
        Adt5::C0(_, _, h1431, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1431
        }
    }, match v2.clone() {
        Adt6::C0(h1432, _, _, _) => {
            h1432
        }
    }, match v2.clone() {
        Adt6::C0(_, h1433, _, _) => {
            h1433
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, h1434, _, _, _, _, _, _, _, _, _, _) => {
            h1434
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, h1435, _, _, _, _, _, _, _, _, _) => {
            h1435
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, h1436, _, _, _, _, _, _, _, _) => {
            h1436
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, h1437, _, _, _, _, _, _, _) => {
            h1437
        }
    }, match v2.clone() {
        Adt6::C0(_, _, h1438, _) => {
            h1438
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, h1439, _, _, _, _, _) => {
            h1439
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h1440, _, _, _, _) => {
            h1440
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1441, _, _, _) => {
            h1441
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1442, _, _) => {
            h1442
        }
    }, match v2 {
        Adt6::C0(_, _, _, h1443) => {
            h1443
        }
    }, match v0 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1444) => {
            h1444
        }
    })
}

pub fn f59(v0: Adt5, v1: Adt5) -> Adt5 {
    let v2: Adt6 = f66(match v0.clone() {
        Adt5::C0(_, _, h1445, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1445
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, h1446, _, _, _, _, _, _, _, _) => {
            h1446
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, h1447, _, _, _, _, _, _, _) => {
            h1447
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1448, _, _) => {
            h1448
        }
    }, match v1.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1449, _, _, _) => {
            h1449
        }
    }, match v1 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1450, _, _) => {
            h1450
        }
    });
    Adt5::C0(match v0.clone() {
        Adt5::C0(h1451, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1451
        }
    }, match v0.clone() {
        Adt5::C0(_, h1452, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1452
        }
    }, match v2.clone() {
        Adt6::C0(h1453, _, _, _) => {
            h1453
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, h1454, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1454
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, h1455, _, _, _, _, _, _, _, _, _, _, _) => {
            h1455
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, h1456, _, _, _, _, _, _, _, _, _, _) => {
            h1456
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, h1457, _, _, _, _, _, _, _, _, _) => {
            h1457
        }
    }, match v2.clone() {
        Adt6::C0(_, h1458, _, _) => {
            h1458
        }
    }, match v2.clone() {
        Adt6::C0(_, _, h1459, _) => {
            h1459
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, h1460, _, _, _, _, _, _) => {
            h1460
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, h1461, _, _, _, _, _) => {
            h1461
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h1462, _, _, _, _) => {
            h1462
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1463, _, _, _) => {
            h1463
        }
    }, match v2 {
        Adt6::C0(_, _, _, h1464) => {
            h1464
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1465, _) => {
            h1465
        }
    }, match v0 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1466) => {
            h1466
        }
    })
}

pub fn f60(v0: Adt5, v1: Adt5) -> Adt5 {
    let v2: Adt6 = f66(match v0.clone() {
        Adt5::C0(_, h1467, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1467
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, h1468, _, _, _, _, _, _, _, _, _) => {
            h1468
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h1469, _, _, _, _) => {
            h1469
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1470, _, _, _) => {
            h1470
        }
    }, match v1.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, h1471, _, _, _, _, _) => {
            h1471
        }
    }, match v1 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h1472, _, _, _, _) => {
            h1472
        }
    });
    Adt5::C0(match v0.clone() {
        Adt5::C0(h1473, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1473
        }
    }, match v2.clone() {
        Adt6::C0(h1474, _, _, _) => {
            h1474
        }
    }, match v0.clone() {
        Adt5::C0(_, _, h1475, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1475
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, h1476, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1476
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, h1477, _, _, _, _, _, _, _, _, _, _, _) => {
            h1477
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, h1478, _, _, _, _, _, _, _, _, _, _) => {
            h1478
        }
    }, match v2.clone() {
        Adt6::C0(_, h1479, _, _) => {
            h1479
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, h1480, _, _, _, _, _, _, _, _) => {
            h1480
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, h1481, _, _, _, _, _, _, _) => {
            h1481
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, h1482, _, _, _, _, _, _) => {
            h1482
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, h1483, _, _, _, _, _) => {
            h1483
        }
    }, match v2.clone() {
        Adt6::C0(_, _, h1484, _) => {
            h1484
        }
    }, match v2 {
        Adt6::C0(_, _, _, h1485) => {
            h1485
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1486, _, _) => {
            h1486
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1487, _) => {
            h1487
        }
    }, match v0 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1488) => {
            h1488
        }
    })
}

pub fn f61(v0: Adt5, v1: Adt5) -> Adt5 {
    let v2: Adt6 = f66(match v0.clone() {
        Adt5::C0(h1489, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1489
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, h1490, _, _, _, _, _, _, _, _, _, _) => {
            h1490
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, h1491, _, _, _, _, _) => {
            h1491
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1492) => {
            h1492
        }
    }, match v1.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, h1493, _, _, _, _, _, _, _) => {
            h1493
        }
    }, match v1 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, h1494, _, _, _, _, _, _) => {
            h1494
        }
    });
    Adt5::C0(match v2.clone() {
        Adt6::C0(h1495, _, _, _) => {
            h1495
        }
    }, match v0.clone() {
        Adt5::C0(_, h1496, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1496
        }
    }, match v0.clone() {
        Adt5::C0(_, _, h1497, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1497
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, h1498, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1498
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, h1499, _, _, _, _, _, _, _, _, _, _, _) => {
            h1499
        }
    }, match v2.clone() {
        Adt6::C0(_, h1500, _, _) => {
            h1500
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, h1501, _, _, _, _, _, _, _, _, _) => {
            h1501
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, h1502, _, _, _, _, _, _, _, _) => {
            h1502
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, h1503, _, _, _, _, _, _, _) => {
            h1503
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, h1504, _, _, _, _, _, _) => {
            h1504
        }
    }, match v2.clone() {
        Adt6::C0(_, _, h1505, _) => {
            h1505
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h1506, _, _, _, _) => {
            h1506
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1507, _, _, _) => {
            h1507
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1508, _, _) => {
            h1508
        }
    }, match v0 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1509, _) => {
            h1509
        }
    }, match v2 {
        Adt6::C0(_, _, _, h1510) => {
            h1510
        }
    })
}

pub fn f62(v0: Adt5, v1: Adt5) -> Adt5 {
    let v2: Adt6 = f66(match v0.clone() {
        Adt5::C0(_, _, _, h1511, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1511
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, h1512, _, _, _, _, _, _, _, _) => {
            h1512
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h1513, _, _, _, _) => {
            h1513
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1514) => {
            h1514
        }
    }, match v1.clone() {
        Adt5::C0(_, _, _, _, _, _, h1515, _, _, _, _, _, _, _, _, _) => {
            h1515
        }
    }, match v1 {
        Adt5::C0(_, _, _, _, _, _, _, h1516, _, _, _, _, _, _, _, _) => {
            h1516
        }
    });
    Adt5::C0(match v0.clone() {
        Adt5::C0(h1517, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1517
        }
    }, match v0.clone() {
        Adt5::C0(_, h1518, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1518
        }
    }, match v0.clone() {
        Adt5::C0(_, _, h1519, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1519
        }
    }, match v2.clone() {
        Adt6::C0(h1520, _, _, _) => {
            h1520
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, h1521, _, _, _, _, _, _, _, _, _, _, _) => {
            h1521
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, h1522, _, _, _, _, _, _, _, _, _, _) => {
            h1522
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, h1523, _, _, _, _, _, _, _, _, _) => {
            h1523
        }
    }, match v2.clone() {
        Adt6::C0(_, h1524, _, _) => {
            h1524
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, h1525, _, _, _, _, _, _, _) => {
            h1525
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, h1526, _, _, _, _, _, _) => {
            h1526
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, h1527, _, _, _, _, _) => {
            h1527
        }
    }, match v2.clone() {
        Adt6::C0(_, _, h1528, _) => {
            h1528
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1529, _, _, _) => {
            h1529
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1530, _, _) => {
            h1530
        }
    }, match v0 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1531, _) => {
            h1531
        }
    }, match v2 {
        Adt6::C0(_, _, _, h1532) => {
            h1532
        }
    })
}

pub fn f63(v0: Adt5, v1: Adt5) -> Adt5 {
    let v2: Adt6 = f66(match v0.clone() {
        Adt5::C0(_, _, h1533, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1533
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, h1534, _, _, _, _, _, _, _, _, _) => {
            h1534
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, h1535, _, _, _, _, _) => {
            h1535
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1536, _) => {
            h1536
        }
    }, match v1.clone() {
        Adt5::C0(_, _, _, _, h1537, _, _, _, _, _, _, _, _, _, _, _) => {
            h1537
        }
    }, match v1 {
        Adt5::C0(_, _, _, _, _, h1538, _, _, _, _, _, _, _, _, _, _) => {
            h1538
        }
    });
    Adt5::C0(match v0.clone() {
        Adt5::C0(h1539, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1539
        }
    }, match v0.clone() {
        Adt5::C0(_, h1540, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1540
        }
    }, match v2.clone() {
        Adt6::C0(h1541, _, _, _) => {
            h1541
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, h1542, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1542
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, h1543, _, _, _, _, _, _, _, _, _, _, _) => {
            h1543
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, h1544, _, _, _, _, _, _, _, _, _, _) => {
            h1544
        }
    }, match v2.clone() {
        Adt6::C0(_, h1545, _, _) => {
            h1545
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, h1546, _, _, _, _, _, _, _, _) => {
            h1546
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, h1547, _, _, _, _, _, _, _) => {
            h1547
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, h1548, _, _, _, _, _, _) => {
            h1548
        }
    }, match v2.clone() {
        Adt6::C0(_, _, h1549, _) => {
            h1549
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h1550, _, _, _, _) => {
            h1550
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1551, _, _, _) => {
            h1551
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1552, _, _) => {
            h1552
        }
    }, match v2 {
        Adt6::C0(_, _, _, h1553) => {
            h1553
        }
    }, match v0 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1554) => {
            h1554
        }
    })
}

pub fn f64(v0: Adt5, v1: Adt5) -> Adt5 {
    let v2: Adt6 = f66(match v0.clone() {
        Adt5::C0(_, h1555, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1555
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, h1556, _, _, _, _, _, _, _, _, _, _) => {
            h1556
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, h1557, _, _, _, _, _, _) => {
            h1557
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1558, _, _) => {
            h1558
        }
    }, match v1.clone() {
        Adt5::C0(_, _, h1559, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1559
        }
    }, match v1 {
        Adt5::C0(_, _, _, h1560, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1560
        }
    });
    Adt5::C0(match v0.clone() {
        Adt5::C0(h1561, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1561
        }
    }, match v2.clone() {
        Adt6::C0(h1562, _, _, _) => {
            h1562
        }
    }, match v0.clone() {
        Adt5::C0(_, _, h1563, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1563
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, h1564, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1564
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, h1565, _, _, _, _, _, _, _, _, _, _, _) => {
            h1565
        }
    }, match v2.clone() {
        Adt6::C0(_, h1566, _, _) => {
            h1566
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, h1567, _, _, _, _, _, _, _, _, _) => {
            h1567
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, h1568, _, _, _, _, _, _, _, _) => {
            h1568
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, h1569, _, _, _, _, _, _, _) => {
            h1569
        }
    }, match v2.clone() {
        Adt6::C0(_, _, h1570, _) => {
            h1570
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, h1571, _, _, _, _, _) => {
            h1571
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h1572, _, _, _, _) => {
            h1572
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1573, _, _, _) => {
            h1573
        }
    }, match v2 {
        Adt6::C0(_, _, _, h1574) => {
            h1574
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1575, _) => {
            h1575
        }
    }, match v0 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1576) => {
            h1576
        }
    })
}

pub fn f65(v0: Adt5, v1: Adt5) -> Adt5 {
    let v2: Adt6 = f66(match v0.clone() {
        Adt5::C0(h1577, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1577
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, h1578, _, _, _, _, _, _, _, _, _, _, _) => {
            h1578
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, h1579, _, _, _, _, _, _, _) => {
            h1579
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1580, _, _, _) => {
            h1580
        }
    }, match v1.clone() {
        Adt5::C0(h1581, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1581
        }
    }, match v1 {
        Adt5::C0(_, h1582, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1582
        }
    });
    Adt5::C0(match v2.clone() {
        Adt6::C0(h1583, _, _, _) => {
            h1583
        }
    }, match v0.clone() {
        Adt5::C0(_, h1584, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1584
        }
    }, match v0.clone() {
        Adt5::C0(_, _, h1585, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1585
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, h1586, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1586
        }
    }, match v2.clone() {
        Adt6::C0(_, h1587, _, _) => {
            h1587
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, h1588, _, _, _, _, _, _, _, _, _, _) => {
            h1588
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, h1589, _, _, _, _, _, _, _, _, _) => {
            h1589
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, h1590, _, _, _, _, _, _, _, _) => {
            h1590
        }
    }, match v2.clone() {
        Adt6::C0(_, _, h1591, _) => {
            h1591
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, h1592, _, _, _, _, _, _) => {
            h1592
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, h1593, _, _, _, _, _) => {
            h1593
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h1594, _, _, _, _) => {
            h1594
        }
    }, match v2 {
        Adt6::C0(_, _, _, h1595) => {
            h1595
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1596, _, _) => {
            h1596
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1597, _) => {
            h1597
        }
    }, match v0 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1598) => {
            h1598
        }
    })
}

pub fn f66(v0: u32, v1: u32, v2: u32, v3: u32, v4: u32, v5: u32) -> Adt6 {
    let v6: u32 = f67(f67(v0, v1), v4);
    let v7: u32 = f68(f44(v3, v6), 16u32, 16u32);
    let v8: u32 = f67(v2, v7);
    let v9: u32 = f68(f44(v1, v8), 12u32, 20u32);
    let v10: u32 = f67(f67(v6, v9), v5);
    let v11: u32 = f68(f44(v7, v10), 8u32, 24u32);
    let v12: u32 = f67(v8, v11);
    let v13: u32 = f68(f44(v9, v12), 7u32, 25u32);
    Adt6::C0(v10, v13, v12, v11)
}

pub fn f67(v0: u32, v1: u32) -> u32 {
    let m1601 = {
        let a1599 = f69(v0);
        let a1600 = f69(v1);
        fixed_u64::checked_add(a1599, a1600)
    };
    match m1601 {
        None => {
            0u32
        }
        Some(v2) => {
            f70(v2)
        }
    }
}

pub fn f68(v0: u32, v1: u32, v2: u32) -> u32 {
    let a1602 = f56(v0, v1);
    let a1603 = f52(v0, v2);
    fixed_u32::bit_or(a1602, a1603)
}

pub fn f69(v0: u32) -> u64 {
    let m1605 = {
        let a1604 = v0;
        fixed_u64::convert(i128::from(a1604))
    };
    match m1605 {
        None => {
            0u64
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f70(v0: u64) -> u32 {
    let m1609 = {
        let a1608 = {
            let a1606 = v0;
            let a1607 = 4294967295u64;
            fixed_u64::bit_and(a1606, a1607)
        };
        fixed_u32::convert(i128::from(a1608))
    };
    match m1609 {
        None => {
            0u32
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn run(p0: Adt0, p1: List<Bytes>, p2: u64) -> R<Result<Bytes, Adt1>> {
    f0(p0, p1, p2)
}
