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
    C0(u64),
}

#[derive(Clone)]
pub enum Adt1 {
    C0,
    C1(Bytes),
    C2(Bytes),
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
    F13(u32, Bytes, Adt3, Adt2),
}

impl Fn0 {
    pub fn apply(&self, p0: (u64, List<Adt3>)) -> R<(u64, List<Adt3>)> {
        match self {
            Fn0::F13(k0, k1, k2, k3) => f13(*k0, k1.clone(), k2.clone(), k3.clone(), p0),
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
    F34(u32, u64, u32, u64, u32, Bytes),
}

impl Fn3 {
    pub fn apply(&self, p0: (u64, Adt3)) -> R<(u64, Adt3)> {
        match self {
            Fn3::F34(k0, k1, k2, k3, k4, k5) => f34(*k0, *k1, *k2, *k3, *k4, k5.clone(), p0),
        }
    }
}

#[derive(Clone)]
pub enum Fn4 {
    F39(u32, Adt3),
}

impl Fn4 {
    pub fn apply(&self, p0: (Adt3, (u64, List<Adt3>))) -> Option<(Adt3, (u64, List<Adt3>))> {
        match self {
            Fn4::F39(k0, k1) => f39(*k0, k1.clone(), p0),
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

pub fn f0(v0: Bytes, v1: Bytes, v2: u64) -> R<Result<Bytes, Adt0>> {
    let m1 = f1(Adt1::C1(v0))?;
    match m1 {
        Ok(v3) => {
            Ok(Ok::<Bytes, Adt0>(f2(f3(v3, v1)?, v2)?))
        }
        Err(v4) => {
            Ok(Err::<Bytes, Adt0>(v4))
        }
    }
}

pub fn f1(v0: Adt1) -> R<Result<Adt2, Adt0>> {
    let m2 = v0;
    match m2 {
        Adt1::C0 => {
            Ok(Ok::<Adt2, Adt0>(f4(f5(), 0u32)))
        }
        Adt1::C1(v1) => {
            let a4 = {
                let a3 = v1.clone();
                length_bytes(a3)
            };
            let a5 = 32u64;
            if nat_eq(a4, a5) {
                Ok(Ok::<Adt2, Adt0>(f4(f6(v1)?, f7())))
            } else {
                Ok(Err::<Adt2, Adt0>(Adt0::C0({
                    let a6 = v1;
                    length_bytes(a6)
                })))
            }
        }
        Adt1::C2(v2) => {
            Ok(Ok::<Adt2, Adt0>(f4(f8(v2)?, f9())))
        }
    }
}

pub fn f2(v0: Adt2, v1: u64) -> R<Bytes> {
    f10(f11(v0)?, v1)
}

pub fn f3(v0: Adt2, v1: Bytes) -> R<Adt2> {
    let v2: Bytes = {
        let a8 = match v0.clone() {
            Adt2::C0(_, _, _, h7, _) => {
                h7
            }
        };
        let a9 = v1;
        append_bytes(a8, a9)
    };
    let v3: u64 = {
        let a10 = v2.clone();
        length_bytes(a10)
    };
    let v4: u64 = {
        let a11 = v3;
        let a12 = 0u64;
        if nat_eq(a11, a12) {
            0u64
        } else {
            let a15 = {
                let a13 = v3;
                let a14 = 1u64;
                nat_sub(a13, a14)
            };
            let a16 = 1024u64;
            let a17 = 0u64;
            nat_quot(a15, a16, a17)
        }
    };
    let v5: u32 = match v0.clone() {
        Adt2::C0(_, h18, _, _, _) => {
            h18
        }
    };
    let v6: Adt3 = match v0.clone() {
        Adt2::C0(h19, _, _, _, _) => {
            h19
        }
    };
    let v7: (u64, List<Adt3>) = f12(Fn0::F13(v5, v2.clone(), v6.clone(), v0.clone()), v4, (0u64, match v0.clone() {
        Adt2::C0(_, _, _, _, h20) => {
            h20
        }
    }))?;
    let v8: u64 = {
        let a21 = v4;
        let a22 = 1024u64;
        nat_mul(a21, a22)?
    };
    Ok(Adt2::C0(v6, v5, {
        let a24 = match v0 {
            Adt2::C0(_, _, h23, _, _) => {
                h23
            }
        };
        let a25 = v4;
        nat_add(a24, a25)?
    }, {
        let m31 = {
            let a28 = v2;
            let a29 = v8;
            let a30 = {
                let a26 = v3;
                let a27 = v8;
                nat_sub(a26, a27)
            };
            slice_bytes(a28, a29, a30)
        };
        match m31 {
            None => {
                Bytes::lit(&[])
            }
            Some(v9) => {
                v9
            }
        }
    }, {
        let (_, h32) = v7;
        h32
    }))
}

pub fn f4(v0: Adt3, v1: u32) -> Adt2 {
    Adt2::C0(v0, v1, 0u64, Bytes::lit(&[]), List::<Adt3>::nil())
}

pub fn f5() -> Adt3 {
    Adt3::C0(1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, 1359893119u32, 2600822924u32, 528734635u32, 1541459225u32)
}

pub fn f6(v0: Bytes) -> R<Adt3> {
    Ok(f14(f15(v0.clone(), {
        let a33 = v0;
        length_bytes(a33)
    }, 0u64)?))
}

pub fn f7() -> u32 {
    16u32
}

pub fn f8(v0: Bytes) -> R<Adt3> {
    Ok(f14(f16(f11(f3(f4(f5(), f17()), v0)?)?, 0u64)))
}

pub fn f9() -> u32 {
    64u32
}

pub fn f10(v0: Adt4, v1: u64) -> R<Bytes> {
    let v2: u64 = {
        let a36 = {
            let a34 = v1;
            let a35 = 63u64;
            nat_add(a34, a35)?
        };
        let a37 = 64u64;
        let a38 = 0u64;
        nat_quot(a36, a37, a38)
    };
    let v3: (u64, Bytes) = f18(Fn1::F19(v0), v2, (0u64, Bytes::lit(&[])))?;
    let m43 = {
        let a40 = {
            let (_, h39) = v3.clone();
            h39
        };
        let a41 = 0u64;
        let a42 = v1;
        slice_bytes(a40, a41, a42)
    };
    match m43 {
        None => {
            let (_, h44) = v3;
            Ok(h44)
        }
        Some(v4) => {
            Ok(v4)
        }
    }
}

pub fn f11(v0: Adt2) -> R<Adt4> {
    let v1: Adt3 = match v0.clone() {
        Adt2::C0(h45, _, _, _, _) => {
            h45
        }
    };
    let v2: u32 = match v0.clone() {
        Adt2::C0(_, h46, _, _, _) => {
            h46
        }
    };
    let v3: Bytes = match v0.clone() {
        Adt2::C0(_, _, _, h47, _) => {
            h47
        }
    };
    Ok(f20(Fn2::F21(v2, v1.clone()), f22(v1, v2, v3.clone(), 0u64, {
        let a48 = v3;
        length_bytes(a48)
    }, match v0.clone() {
        Adt2::C0(_, _, h49, _, _) => {
            h49
        }
    })?, match v0 {
        Adt2::C0(_, _, _, _, h50) => {
            h50
        }
    }))
}

pub fn f12(v0: Fn0, p54: u64, p55: (u64, List<Adt3>)) -> R<(u64, List<Adt3>)> {
    let m53 = (p54, p55);
    {
        let mut m53 = m53;
        loop {
            let step = {
                let (v1, v2) = m53;
                let m51 = v1;
                if m51 == 0 {
                    Err::<(u64, (u64, List<Adt3>)), R<(u64, List<Adt3>)>>(Ok(v2))
                } else {
                    let v3 = m51 - 1;
                    Ok::<(u64, (u64, List<Adt3>)), R<(u64, List<Adt3>)>>((v3, {
                        let c52: Fn0 = v0.clone();
                        c52.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m53 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f13(v0: u32, v1: Bytes, v2: Adt3, v3: Adt2, v4: (u64, List<Adt3>)) -> R<(u64, List<Adt3>)> {
    let v5: u64 = {
        let a58 = match v3 {
            Adt2::C0(_, _, h56, _, _) => {
                h56
            }
        };
        let a59 = {
            let (h57, _) = v4.clone();
            h57
        };
        nat_add(a58, a59)?
    };
    Ok(({
        let a61 = {
            let (h60, _) = v4.clone();
            h60
        };
        let a62 = 1u64;
        nat_add(a61, a62)?
    }, f23(v2.clone(), v0, {
        let (_, h63) = v4;
        h63
    }, f24(f22(v2, v0, v1, {
        let a65 = {
            let (h64, _) = v4;
            h64
        };
        let a66 = 1024u64;
        nat_mul(a65, a66)?
    }, 1024u64, v5)?), {
        let a67 = v5;
        let a68 = 1u64;
        nat_add(a67, a68)?
    })))
}

pub fn f14(v0: Adt5) -> Adt3 {
    Adt3::C0(match v0.clone() {
        Adt5::C0(h69, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h69
        }
    }, match v0.clone() {
        Adt5::C0(_, h70, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h70
        }
    }, match v0.clone() {
        Adt5::C0(_, _, h71, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h71
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, h72, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h72
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, h73, _, _, _, _, _, _, _, _, _, _, _) => {
            h73
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, h74, _, _, _, _, _, _, _, _, _, _) => {
            h74
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, h75, _, _, _, _, _, _, _, _, _) => {
            h75
        }
    }, match v0 {
        Adt5::C0(_, _, _, _, _, _, _, h76, _, _, _, _, _, _, _, _) => {
            h76
        }
    })
}

pub fn f15(v0: Bytes, v1: u64, v2: u64) -> R<Adt5> {
    Ok(Adt5::C0(f25(v0.clone(), v1, {
        let a77 = v2;
        let a78 = 0u64;
        nat_add(a77, a78)?
    })?, f25(v0.clone(), v1, {
        let a79 = v2;
        let a80 = 4u64;
        nat_add(a79, a80)?
    })?, f25(v0.clone(), v1, {
        let a81 = v2;
        let a82 = 8u64;
        nat_add(a81, a82)?
    })?, f25(v0.clone(), v1, {
        let a83 = v2;
        let a84 = 12u64;
        nat_add(a83, a84)?
    })?, f25(v0.clone(), v1, {
        let a85 = v2;
        let a86 = 16u64;
        nat_add(a85, a86)?
    })?, f25(v0.clone(), v1, {
        let a87 = v2;
        let a88 = 20u64;
        nat_add(a87, a88)?
    })?, f25(v0.clone(), v1, {
        let a89 = v2;
        let a90 = 24u64;
        nat_add(a89, a90)?
    })?, f25(v0.clone(), v1, {
        let a91 = v2;
        let a92 = 28u64;
        nat_add(a91, a92)?
    })?, f25(v0.clone(), v1, {
        let a93 = v2;
        let a94 = 32u64;
        nat_add(a93, a94)?
    })?, f25(v0.clone(), v1, {
        let a95 = v2;
        let a96 = 36u64;
        nat_add(a95, a96)?
    })?, f25(v0.clone(), v1, {
        let a97 = v2;
        let a98 = 40u64;
        nat_add(a97, a98)?
    })?, f25(v0.clone(), v1, {
        let a99 = v2;
        let a100 = 44u64;
        nat_add(a99, a100)?
    })?, f25(v0.clone(), v1, {
        let a101 = v2;
        let a102 = 48u64;
        nat_add(a101, a102)?
    })?, f25(v0.clone(), v1, {
        let a103 = v2;
        let a104 = 52u64;
        nat_add(a103, a104)?
    })?, f25(v0.clone(), v1, {
        let a105 = v2;
        let a106 = 56u64;
        nat_add(a105, a106)?
    })?, f25(v0, v1, {
        let a107 = v2;
        let a108 = 60u64;
        nat_add(a107, a108)?
    })?))
}

pub fn f16(v0: Adt4, v1: u64) -> Adt5 {
    f26(match v0.clone() {
        Adt4::C0(h109, _, _, _, _) => {
            h109
        }
    }, match v0.clone() {
        Adt4::C0(_, h110, _, _, _) => {
            h110
        }
    }, f27(v1), f28(v1), match v0.clone() {
        Adt4::C0(_, _, _, h111, _) => {
            h111
        }
    }, {
        let a113 = match v0 {
            Adt4::C0(_, _, _, _, h112) => {
                h112
            }
        };
        let a114 = f29();
        fixed_u32::bit_or(a113, a114)
    })
}

pub fn f17() -> u32 {
    32u32
}

pub fn f18(v0: Fn1, p118: u64, p119: (u64, Bytes)) -> R<(u64, Bytes)> {
    let m117 = (p118, p119);
    {
        let mut m117 = m117;
        loop {
            let step = {
                let (v1, v2) = m117;
                let m115 = v1;
                if m115 == 0 {
                    Err::<(u64, (u64, Bytes)), R<(u64, Bytes)>>(Ok(v2))
                } else {
                    let v3 = m115 - 1;
                    Ok::<(u64, (u64, Bytes)), R<(u64, Bytes)>>((v3, {
                        let c116: Fn1 = v0.clone();
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

pub fn f19(v0: Adt4, v1: (u64, Bytes)) -> R<(u64, Bytes)> {
    Ok(({
        let a121 = {
            let (h120, _) = v1.clone();
            h120
        };
        let a122 = 1u64;
        nat_add(a121, a122)?
    }, {
        let a125 = {
            let (_, h123) = v1;
            h123
        };
        let a126 = f30(f16(v0, {
            let (h124, _) = v1;
            h124
        }));
        append_bytes(a125, a126)
    }))
}

pub fn f20(v0: Fn2, p130: Adt4, p131: List<Adt3>) -> Adt4 {
    let m129 = (p130, p131);
    {
        let mut m129 = m129;
        loop {
            let step = {
                let (v1, v2) = m129;
                let m127 = v2;
                match m127.uncons() {
                    None => {
                        Err::<(Adt4, List<Adt3>), Adt4>(v1)
                    }
                    Some((v3, v4)) => {
                        Ok::<(Adt4, List<Adt3>), Adt4>(({
                            let c128: Fn2 = v0.clone();
                            c128.apply(v1, v3)
                        }, v4))
                    }
                }
            };
            match step {
                Ok(next) => { m129 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f21(v0: u32, v1: Adt3, v2: Adt4, v3: Adt3) -> Adt4 {
    f31(v3, f24(v2), v1, v0)
}

pub fn f22(v0: Adt3, v1: u32, v2: Bytes, v3: u64, v4: u64, v5: u64) -> R<Adt4> {
    let v6: u64 = {
        let a132 = v3;
        let a133 = v4;
        nat_add(a132, a133)?
    };
    let v7: u64 = f32(v4)?;
    let v8: u32 = f27(v5);
    let v9: u32 = f28(v5);
    let v10: (u64, Adt3) = f33(Fn3::F34(v1, v3, v9, v6, v8, v2.clone()), {
        let a134 = v7;
        let a135 = 64u64;
        let a136 = 0u64;
        nat_quot(a134, a135, a136)
    }, (0u64, v0))?;
    Ok(Adt4::C0({
        let (_, h137) = v10;
        h137
    }, f15(v2, v6, {
        let a138 = v3;
        let a139 = v7;
        nat_add(a138, a139)?
    })?, v5, f35({
        let a140 = v4;
        let a141 = v7;
        nat_sub(a140, a141)
    }, 7u64), {
        let a144 = v1;
        let a145 = {
            let a142 = f36();
            let a143 = f37(v7);
            fixed_u32::bit_or(a142, a143)
        };
        fixed_u32::bit_or(a144, a145)
    }))
}

pub fn f23(v0: Adt3, v1: u32, v2: List<Adt3>, v3: Adt3, v4: u64) -> List<Adt3> {
    let v5: (Adt3, (u64, List<Adt3>)) = {
        let (h146, _) = f38(Fn4::F39(v1, v0), 64u64, (v3, (v4, v2)));
        h146
    };
    List::cons({
        let (h147, _) = v5;
        h147
    }, {
        let (_, h148) = {
            let (_, h149) = v5;
            h149
        };
        h148
    })
}

pub fn f24(v0: Adt4) -> Adt3 {
    f14(f26(match v0.clone() {
        Adt4::C0(h150, _, _, _, _) => {
            h150
        }
    }, match v0.clone() {
        Adt4::C0(_, h151, _, _, _) => {
            h151
        }
    }, f27(match v0.clone() {
        Adt4::C0(_, _, h152, _, _) => {
            h152
        }
    }), f28(match v0.clone() {
        Adt4::C0(_, _, h153, _, _) => {
            h153
        }
    }), match v0.clone() {
        Adt4::C0(_, _, _, h154, _) => {
            h154
        }
    }, match v0 {
        Adt4::C0(_, _, _, _, h155) => {
            h155
        }
    }))
}

pub fn f25(v0: Bytes, v1: u64, v2: u64) -> R<u32> {
    Ok(f40(f41(v0.clone(), v1, v2), f41(v0.clone(), v1, {
        let a156 = v2;
        let a157 = 1u64;
        nat_add(a156, a157)?
    }), f41(v0.clone(), v1, {
        let a158 = v2;
        let a159 = 2u64;
        nat_add(a158, a159)?
    }), f41(v0, v1, {
        let a160 = v2;
        let a161 = 3u64;
        nat_add(a160, a161)?
    })))
}

pub fn f26(v0: Adt3, v1: Adt5, v2: u32, v3: u32, v4: u32, v5: u32) -> Adt5 {
    let v6: Adt5 = {
        let (h162, _) = f42(Fn5::F43, 7u64, (Adt5::C0(match v0.clone() {
            Adt3::C0(h163, _, _, _, _, _, _, _) => {
                h163
            }
        }, match v0.clone() {
            Adt3::C0(_, h164, _, _, _, _, _, _) => {
                h164
            }
        }, match v0.clone() {
            Adt3::C0(_, _, h165, _, _, _, _, _) => {
                h165
            }
        }, match v0.clone() {
            Adt3::C0(_, _, _, h166, _, _, _, _) => {
                h166
            }
        }, match v0.clone() {
            Adt3::C0(_, _, _, _, h167, _, _, _) => {
                h167
            }
        }, match v0.clone() {
            Adt3::C0(_, _, _, _, _, h168, _, _) => {
                h168
            }
        }, match v0.clone() {
            Adt3::C0(_, _, _, _, _, _, h169, _) => {
                h169
            }
        }, match v0.clone() {
            Adt3::C0(_, _, _, _, _, _, _, h170) => {
                h170
            }
        }, 1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32, v2, v3, v4, v5), v1));
        h162
    };
    Adt5::C0(f44(match v6.clone() {
        Adt5::C0(h171, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h171
        }
    }, match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, h172, _, _, _, _, _, _, _) => {
            h172
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, h173, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h173
        }
    }, match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, h174, _, _, _, _, _, _) => {
            h174
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, h175, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h175
        }
    }, match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, h176, _, _, _, _, _) => {
            h176
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, _, h177, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h177
        }
    }, match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h178, _, _, _, _) => {
            h178
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, _, _, h179, _, _, _, _, _, _, _, _, _, _, _) => {
            h179
        }
    }, match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h180, _, _, _) => {
            h180
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, _, _, _, h181, _, _, _, _, _, _, _, _, _, _) => {
            h181
        }
    }, match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h182, _, _) => {
            h182
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, h183, _, _, _, _, _, _, _, _, _) => {
            h183
        }
    }, match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h184, _) => {
            h184
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, h185, _, _, _, _, _, _, _, _) => {
            h185
        }
    }, match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h186) => {
            h186
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, h187, _, _, _, _, _, _, _) => {
            h187
        }
    }, match v0.clone() {
        Adt3::C0(h188, _, _, _, _, _, _, _) => {
            h188
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, h189, _, _, _, _, _, _) => {
            h189
        }
    }, match v0.clone() {
        Adt3::C0(_, h190, _, _, _, _, _, _) => {
            h190
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, h191, _, _, _, _, _) => {
            h191
        }
    }, match v0.clone() {
        Adt3::C0(_, _, h192, _, _, _, _, _) => {
            h192
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h193, _, _, _, _) => {
            h193
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h194, _, _, _, _) => {
            h194
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h195, _, _, _) => {
            h195
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h196, _, _, _) => {
            h196
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h197, _, _) => {
            h197
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h198, _, _) => {
            h198
        }
    }), f44(match v6.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h199, _) => {
            h199
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h200, _) => {
            h200
        }
    }), f44(match v6 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h201) => {
            h201
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, h202) => {
            h202
        }
    }))
}

pub fn f27(v0: u64) -> u32 {
    f35({
        let a203 = v0;
        let a204 = 4294967296u64;
        let a205 = 0u64;
        nat_rem(a203, a204, a205)
    }, 32u64)
}

pub fn f28(v0: u64) -> u32 {
    let a206 = v0;
    let a207 = 4294967296u64;
    if nat_lt(a206, a207) {
        0u32
    } else {
        f35({
            let a208 = v0;
            let a209 = 4294967296u64;
            let a210 = 0u64;
            nat_quot(a208, a209, a210)
        }, 32u64)
    }
}

pub fn f29() -> u32 {
    8u32
}

pub fn f30(v0: Adt5) -> Bytes {
    let a255 = f45(match v0.clone() {
        Adt5::C0(h211, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h211
        }
    });
    let a256 = {
        let a253 = f45(match v0.clone() {
            Adt5::C0(_, h212, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                h212
            }
        });
        let a254 = {
            let a251 = f45(match v0.clone() {
                Adt5::C0(_, _, h213, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                    h213
                }
            });
            let a252 = {
                let a249 = f45(match v0.clone() {
                    Adt5::C0(_, _, _, h214, _, _, _, _, _, _, _, _, _, _, _, _) => {
                        h214
                    }
                });
                let a250 = {
                    let a247 = f45(match v0.clone() {
                        Adt5::C0(_, _, _, _, h215, _, _, _, _, _, _, _, _, _, _, _) => {
                            h215
                        }
                    });
                    let a248 = {
                        let a245 = f45(match v0.clone() {
                            Adt5::C0(_, _, _, _, _, h216, _, _, _, _, _, _, _, _, _, _) => {
                                h216
                            }
                        });
                        let a246 = {
                            let a243 = f45(match v0.clone() {
                                Adt5::C0(_, _, _, _, _, _, h217, _, _, _, _, _, _, _, _, _) => {
                                    h217
                                }
                            });
                            let a244 = {
                                let a241 = f45(match v0.clone() {
                                    Adt5::C0(_, _, _, _, _, _, _, h218, _, _, _, _, _, _, _, _) => {
                                        h218
                                    }
                                });
                                let a242 = {
                                    let a239 = f45(match v0.clone() {
                                        Adt5::C0(_, _, _, _, _, _, _, _, h219, _, _, _, _, _, _, _) => {
                                            h219
                                        }
                                    });
                                    let a240 = {
                                        let a237 = f45(match v0.clone() {
                                            Adt5::C0(_, _, _, _, _, _, _, _, _, h220, _, _, _, _, _, _) => {
                                                h220
                                            }
                                        });
                                        let a238 = {
                                            let a235 = f45(match v0.clone() {
                                                Adt5::C0(_, _, _, _, _, _, _, _, _, _, h221, _, _, _, _, _) => {
                                                    h221
                                                }
                                            });
                                            let a236 = {
                                                let a233 = f45(match v0.clone() {
                                                    Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h222, _, _, _, _) => {
                                                        h222
                                                    }
                                                });
                                                let a234 = {
                                                    let a231 = f45(match v0.clone() {
                                                        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h223, _, _, _) => {
                                                            h223
                                                        }
                                                    });
                                                    let a232 = {
                                                        let a229 = f45(match v0.clone() {
                                                            Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h224, _, _) => {
                                                                h224
                                                            }
                                                        });
                                                        let a230 = {
                                                            let a227 = f45(match v0.clone() {
                                                                Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h225, _) => {
                                                                    h225
                                                                }
                                                            });
                                                            let a228 = f45(match v0 {
                                                                Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h226) => {
                                                                    h226
                                                                }
                                                            });
                                                            append_bytes(a227, a228)
                                                        };
                                                        append_bytes(a229, a230)
                                                    };
                                                    append_bytes(a231, a232)
                                                };
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
                    };
                    append_bytes(a247, a248)
                };
                append_bytes(a249, a250)
            };
            append_bytes(a251, a252)
        };
        append_bytes(a253, a254)
    };
    append_bytes(a255, a256)
}

pub fn f31(v0: Adt3, v1: Adt3, v2: Adt3, v3: u32) -> Adt4 {
    Adt4::C0(v2, Adt5::C0(match v0.clone() {
        Adt3::C0(h257, _, _, _, _, _, _, _) => {
            h257
        }
    }, match v0.clone() {
        Adt3::C0(_, h258, _, _, _, _, _, _) => {
            h258
        }
    }, match v0.clone() {
        Adt3::C0(_, _, h259, _, _, _, _, _) => {
            h259
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, h260, _, _, _, _) => {
            h260
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, h261, _, _, _) => {
            h261
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, h262, _, _) => {
            h262
        }
    }, match v0.clone() {
        Adt3::C0(_, _, _, _, _, _, h263, _) => {
            h263
        }
    }, match v0 {
        Adt3::C0(_, _, _, _, _, _, _, h264) => {
            h264
        }
    }, match v1.clone() {
        Adt3::C0(h265, _, _, _, _, _, _, _) => {
            h265
        }
    }, match v1.clone() {
        Adt3::C0(_, h266, _, _, _, _, _, _) => {
            h266
        }
    }, match v1.clone() {
        Adt3::C0(_, _, h267, _, _, _, _, _) => {
            h267
        }
    }, match v1.clone() {
        Adt3::C0(_, _, _, h268, _, _, _, _) => {
            h268
        }
    }, match v1.clone() {
        Adt3::C0(_, _, _, _, h269, _, _, _) => {
            h269
        }
    }, match v1.clone() {
        Adt3::C0(_, _, _, _, _, h270, _, _) => {
            h270
        }
    }, match v1.clone() {
        Adt3::C0(_, _, _, _, _, _, h271, _) => {
            h271
        }
    }, match v1 {
        Adt3::C0(_, _, _, _, _, _, _, h272) => {
            h272
        }
    }), 0u64, 64u32, {
        let a273 = v3;
        let a274 = f46();
        fixed_u32::bit_or(a273, a274)
    })
}

pub fn f32(v0: u64) -> R<u64> {
    let a275 = v0;
    let a276 = 0u64;
    if nat_eq(a275, a276) {
        Ok(0u64)
    } else {
        let a282 = {
            let a279 = {
                let a277 = v0;
                let a278 = 1u64;
                nat_sub(a277, a278)
            };
            let a280 = 64u64;
            let a281 = 0u64;
            nat_quot(a279, a280, a281)
        };
        let a283 = 64u64;
        nat_mul(a282, a283)
    }
}

pub fn f33(v0: Fn3, p287: u64, p288: (u64, Adt3)) -> R<(u64, Adt3)> {
    let m286 = (p287, p288);
    {
        let mut m286 = m286;
        loop {
            let step = {
                let (v1, v2) = m286;
                let m284 = v1;
                if m284 == 0 {
                    Err::<(u64, (u64, Adt3)), R<(u64, Adt3)>>(Ok(v2))
                } else {
                    let v3 = m284 - 1;
                    Ok::<(u64, (u64, Adt3)), R<(u64, Adt3)>>((v3, {
                        let c285: Fn3 = v0.clone();
                        c285.apply(v2)?
                    }))
                }
            };
            match step {
                Ok(next) => { m286 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f34(v0: u32, v1: u64, v2: u32, v3: u64, v4: u32, v5: Bytes, v6: (u64, Adt3)) -> R<(u64, Adt3)> {
    let v7: u64 = {
        let (h289, _) = v6;
        h289
    };
    Ok(({
        let a290 = v7;
        let a291 = 64u64;
        nat_add(a290, a291)?
    }, f14(f26({
        let (_, h292) = v6;
        h292
    }, f15(v5, v3, {
        let a293 = v1;
        let a294 = v7;
        nat_add(a293, a294)?
    })?, v4, v2, 64u32, {
        let a295 = v0;
        let a296 = f37(v7);
        fixed_u32::bit_or(a295, a296)
    }))))
}

pub fn f35(v0: u64, v1: u64) -> u32 {
    let (_, h297) = {
        let (_, h298) = f47(Fn6::F48, v1, (v0, (1u32, 0u32)));
        h298
    };
    h297
}

pub fn f36() -> u32 {
    2u32
}

pub fn f37(v0: u64) -> u32 {
    let a299 = v0;
    let a300 = 0u64;
    if nat_eq(a299, a300) {
        f49()
    } else {
        0u32
    }
}

pub fn f38(v0: Fn4, p305: u64, p306: (Adt3, (u64, List<Adt3>))) -> ((Adt3, (u64, List<Adt3>)), bool) {
    let m304 = (p305, p306);
    {
        let mut m304 = m304;
        loop {
            let step = {
                let (v1, v2) = m304;
                let m301 = v1;
                if m301 == 0 {
                    Err::<(u64, (Adt3, (u64, List<Adt3>))), ((Adt3, (u64, List<Adt3>)), bool)>((v2, false))
                } else {
                    let v3 = m301 - 1;
                    let m303 = {
                        let c302: Fn4 = v0.clone();
                        c302.apply(v2.clone())
                    };
                    match m303 {
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
                Ok(next) => { m304 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f39(v0: u32, v1: Adt3, v2: (Adt3, (u64, List<Adt3>))) -> Option<(Adt3, (u64, List<Adt3>))> {
    f50(v1, v0, v2)
}

pub fn f40(v0: u32, v1: u32, v2: u32, v3: u32) -> u32 {
    let a311 = v0;
    let a312 = {
        let a309 = f51(v1, 8u32);
        let a310 = {
            let a307 = f51(v2, 16u32);
            let a308 = f51(v3, 24u32);
            fixed_u32::bit_or(a307, a308)
        };
        fixed_u32::bit_or(a309, a310)
    };
    fixed_u32::bit_or(a311, a312)
}

pub fn f41(v0: Bytes, v1: u64, v2: u64) -> u32 {
    let a313 = v2;
    let a314 = v1;
    if nat_lt(a313, a314) {
        let m317 = {
            let a315 = v0;
            let a316 = v2;
            index_bytes(a315, a316)
        };
        match m317 {
            None => {
                0u32
            }
            Some(v3) => {
                let m319 = {
                    let a318 = v3;
                    fixed_u32::convert(i128::from(a318))
                };
                match m319 {
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

pub fn f42(v0: Fn5, p323: u64, p324: (Adt5, Adt5)) -> (Adt5, Adt5) {
    let m322 = (p323, p324);
    {
        let mut m322 = m322;
        loop {
            let step = {
                let (v1, v2) = m322;
                let m320 = v1;
                if m320 == 0 {
                    Err::<(u64, (Adt5, Adt5)), (Adt5, Adt5)>(v2)
                } else {
                    let v3 = m320 - 1;
                    Ok::<(u64, (Adt5, Adt5)), (Adt5, Adt5)>((v3, {
                        let c321: Fn5 = v0.clone();
                        c321.apply(v2)
                    }))
                }
            };
            match step {
                Ok(next) => { m322 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f43(v0: (Adt5, Adt5)) -> (Adt5, Adt5) {
    (f52({
        let (h325, _) = v0.clone();
        h325
    }, {
        let (_, h326) = v0.clone();
        h326
    }), f53({
        let (_, h327) = v0;
        h327
    }))
}

pub fn f44(v0: u32, v1: u32) -> u32 {
    let a328 = v0;
    let a329 = v1;
    fixed_u32::bit_xor(a328, a329)
}

pub fn f45(v0: u32) -> Bytes {
    let a340 = f54({
        let a330 = v0;
        let a331 = 255u32;
        fixed_u32::bit_and(a330, a331)
    });
    let a341 = {
        let a338 = f54({
            let a332 = f55(v0, 8u32);
            let a333 = 255u32;
            fixed_u32::bit_and(a332, a333)
        });
        let a339 = {
            let a336 = f54({
                let a334 = f55(v0, 16u32);
                let a335 = 255u32;
                fixed_u32::bit_and(a334, a335)
            });
            let a337 = f54(f55(v0, 24u32));
            append_bytes(a336, a337)
        };
        append_bytes(a338, a339)
    };
    append_bytes(a340, a341)
}

pub fn f46() -> u32 {
    4u32
}

pub fn f47(v0: Fn6, p345: u64, p346: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let m344 = (p345, p346);
    {
        let mut m344 = m344;
        loop {
            let step = {
                let (v1, v2) = m344;
                let m342 = v1;
                if m342 == 0 {
                    Err::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>(v2)
                } else {
                    let v3 = m342 - 1;
                    Ok::<(u64, (u64, (u32, u32))), (u64, (u32, u32))>((v3, {
                        let c343: Fn6 = v0.clone();
                        c343.apply(v2)
                    }))
                }
            };
            match step {
                Ok(next) => { m344 = next; }
                Err(done) => break done,
            }
        }
    }
}

pub fn f48(v0: (u64, (u32, u32))) -> (u64, (u32, u32)) {
    let v1: u64 = {
        let (h347, _) = v0;
        h347
    };
    let v2: u32 = {
        let (h348, _) = {
            let (_, h349) = v0;
            h349
        };
        h348
    };
    let v3: u32 = {
        let (_, h350) = {
            let (_, h351) = v0;
            h351
        };
        h350
    };
    ({
        let a352 = v1;
        let a353 = 2u64;
        let a354 = 0u64;
        nat_quot(a352, a353, a354)
    }, (f51(v2, 1u32), {
        let a358 = {
            let a355 = v1;
            let a356 = 2u64;
            let a357 = 0u64;
            nat_rem(a355, a356, a357)
        };
        let a359 = 1u64;
        if nat_eq(a358, a359) {
            let a360 = v3;
            let a361 = v2;
            fixed_u32::bit_or(a360, a361)
        } else {
            v3
        }
    }))
}

pub fn f49() -> u32 {
    1u32
}

pub fn f50(v0: Adt3, v1: u32, v2: (Adt3, (u64, List<Adt3>))) -> Option<(Adt3, (u64, List<Adt3>))> {
    let v3: u64 = {
        let (h362, _) = {
            let (_, h363) = v2.clone();
            h363
        };
        h362
    };
    let a367 = {
        let a364 = v3;
        let a365 = 2u64;
        let a366 = 0u64;
        nat_rem(a364, a365, a366)
    };
    let a368 = 0u64;
    if nat_eq(a367, a368) {
        let m371 = {
            let (_, h369) = {
                let (_, h370) = v2;
                h370
            };
            h369
        };
        match m371.uncons() {
            None => {
                None::<(Adt3, (u64, List<Adt3>))>
            }
            Some((v4, v5)) => {
                Some((f24(f31(v4, {
                    let (h372, _) = v2;
                    h372
                }, v0, v1)), ({
                    let a373 = v3;
                    let a374 = 2u64;
                    let a375 = 0u64;
                    nat_quot(a373, a374, a375)
                }, v5)))
            }
        }
    } else {
        None::<(Adt3, (u64, List<Adt3>))>
    }
}

pub fn f51(v0: u32, v1: u32) -> u32 {
    let m378 = {
        let a376 = v0;
        let a377 = v1;
        fixed_u32::shift_left(a376, a377)
    };
    match m378 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f52(v0: Adt5, v1: Adt5) -> Adt5 {
    f56(f57(f58(f59(f60(f61(f62(f63(v0, v1.clone()), v1.clone()), v1.clone()), v1.clone()), v1.clone()), v1.clone()), v1.clone()), v1)
}

pub fn f53(v0: Adt5) -> Adt5 {
    Adt5::C0(match v0.clone() {
        Adt5::C0(_, _, h379, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h379
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, h380, _, _, _, _, _, _, _, _, _) => {
            h380
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, h381, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h381
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, h382, _, _, _, _, _) => {
            h382
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, h383, _, _, _, _, _, _, _, _) => {
            h383
        }
    }, match v0.clone() {
        Adt5::C0(h384, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h384
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, h385, _, _, _, _, _, _, _, _, _, _, _) => {
            h385
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h386, _, _) => {
            h386
        }
    }, match v0.clone() {
        Adt5::C0(_, h387, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h387
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h388, _, _, _, _) => {
            h388
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h389, _, _, _) => {
            h389
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, h390, _, _, _, _, _, _, _, _, _, _) => {
            h390
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, h391, _, _, _, _, _, _) => {
            h391
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h392, _) => {
            h392
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h393) => {
            h393
        }
    }, match v0 {
        Adt5::C0(_, _, _, _, _, _, _, _, h394, _, _, _, _, _, _, _) => {
            h394
        }
    })
}

pub fn f54(v0: u32) -> Bytes {
    let a397 = {
        let a395 = v0;
        let a396 = 128u32;
        fixed_u32::bit_and(a395, a396)
    };
    let a398 = 0u32;
    if equal(a397, a398) {
        let a401 = {
            let a399 = v0;
            let a400 = 64u32;
            fixed_u32::bit_and(a399, a400)
        };
        let a402 = 0u32;
        if equal(a401, a402) {
            let a405 = {
                let a403 = v0;
                let a404 = 32u32;
                fixed_u32::bit_and(a403, a404)
            };
            let a406 = 0u32;
            if equal(a405, a406) {
                let a409 = {
                    let a407 = v0;
                    let a408 = 16u32;
                    fixed_u32::bit_and(a407, a408)
                };
                let a410 = 0u32;
                if equal(a409, a410) {
                    let a413 = {
                        let a411 = v0;
                        let a412 = 8u32;
                        fixed_u32::bit_and(a411, a412)
                    };
                    let a414 = 0u32;
                    if equal(a413, a414) {
                        let a417 = {
                            let a415 = v0;
                            let a416 = 4u32;
                            fixed_u32::bit_and(a415, a416)
                        };
                        let a418 = 0u32;
                        if equal(a417, a418) {
                            let a421 = {
                                let a419 = v0;
                                let a420 = 2u32;
                                fixed_u32::bit_and(a419, a420)
                            };
                            let a422 = 0u32;
                            if equal(a421, a422) {
                                let a425 = {
                                    let a423 = v0;
                                    let a424 = 1u32;
                                    fixed_u32::bit_and(a423, a424)
                                };
                                let a426 = 0u32;
                                if equal(a425, a426) {
                                    Bytes::lit(&[0x00])
                                } else {
                                    Bytes::lit(&[0x01])
                                }
                            } else {
                                let a429 = {
                                    let a427 = v0;
                                    let a428 = 1u32;
                                    fixed_u32::bit_and(a427, a428)
                                };
                                let a430 = 0u32;
                                if equal(a429, a430) {
                                    Bytes::lit(&[0x02])
                                } else {
                                    Bytes::lit(&[0x03])
                                }
                            }
                        } else {
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
                                    Bytes::lit(&[0x04])
                                } else {
                                    Bytes::lit(&[0x05])
                                }
                            } else {
                                let a441 = {
                                    let a439 = v0;
                                    let a440 = 1u32;
                                    fixed_u32::bit_and(a439, a440)
                                };
                                let a442 = 0u32;
                                if equal(a441, a442) {
                                    Bytes::lit(&[0x06])
                                } else {
                                    Bytes::lit(&[0x07])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0x08])
                                } else {
                                    Bytes::lit(&[0x09])
                                }
                            } else {
                                let a457 = {
                                    let a455 = v0;
                                    let a456 = 1u32;
                                    fixed_u32::bit_and(a455, a456)
                                };
                                let a458 = 0u32;
                                if equal(a457, a458) {
                                    Bytes::lit(&[0x0a])
                                } else {
                                    Bytes::lit(&[0x0b])
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
                                    Bytes::lit(&[0x0c])
                                } else {
                                    Bytes::lit(&[0x0d])
                                }
                            } else {
                                let a469 = {
                                    let a467 = v0;
                                    let a468 = 1u32;
                                    fixed_u32::bit_and(a467, a468)
                                };
                                let a470 = 0u32;
                                if equal(a469, a470) {
                                    Bytes::lit(&[0x0e])
                                } else {
                                    Bytes::lit(&[0x0f])
                                }
                            }
                        }
                    }
                } else {
                    let a473 = {
                        let a471 = v0;
                        let a472 = 8u32;
                        fixed_u32::bit_and(a471, a472)
                    };
                    let a474 = 0u32;
                    if equal(a473, a474) {
                        let a477 = {
                            let a475 = v0;
                            let a476 = 4u32;
                            fixed_u32::bit_and(a475, a476)
                        };
                        let a478 = 0u32;
                        if equal(a477, a478) {
                            let a481 = {
                                let a479 = v0;
                                let a480 = 2u32;
                                fixed_u32::bit_and(a479, a480)
                            };
                            let a482 = 0u32;
                            if equal(a481, a482) {
                                let a485 = {
                                    let a483 = v0;
                                    let a484 = 1u32;
                                    fixed_u32::bit_and(a483, a484)
                                };
                                let a486 = 0u32;
                                if equal(a485, a486) {
                                    Bytes::lit(&[0x10])
                                } else {
                                    Bytes::lit(&[0x11])
                                }
                            } else {
                                let a489 = {
                                    let a487 = v0;
                                    let a488 = 1u32;
                                    fixed_u32::bit_and(a487, a488)
                                };
                                let a490 = 0u32;
                                if equal(a489, a490) {
                                    Bytes::lit(&[0x12])
                                } else {
                                    Bytes::lit(&[0x13])
                                }
                            }
                        } else {
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
                                    Bytes::lit(&[0x14])
                                } else {
                                    Bytes::lit(&[0x15])
                                }
                            } else {
                                let a501 = {
                                    let a499 = v0;
                                    let a500 = 1u32;
                                    fixed_u32::bit_and(a499, a500)
                                };
                                let a502 = 0u32;
                                if equal(a501, a502) {
                                    Bytes::lit(&[0x16])
                                } else {
                                    Bytes::lit(&[0x17])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0x18])
                                } else {
                                    Bytes::lit(&[0x19])
                                }
                            } else {
                                let a517 = {
                                    let a515 = v0;
                                    let a516 = 1u32;
                                    fixed_u32::bit_and(a515, a516)
                                };
                                let a518 = 0u32;
                                if equal(a517, a518) {
                                    Bytes::lit(&[0x1a])
                                } else {
                                    Bytes::lit(&[0x1b])
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
                                    Bytes::lit(&[0x1c])
                                } else {
                                    Bytes::lit(&[0x1d])
                                }
                            } else {
                                let a529 = {
                                    let a527 = v0;
                                    let a528 = 1u32;
                                    fixed_u32::bit_and(a527, a528)
                                };
                                let a530 = 0u32;
                                if equal(a529, a530) {
                                    Bytes::lit(&[0x1e])
                                } else {
                                    Bytes::lit(&[0x1f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a533 = {
                    let a531 = v0;
                    let a532 = 16u32;
                    fixed_u32::bit_and(a531, a532)
                };
                let a534 = 0u32;
                if equal(a533, a534) {
                    let a537 = {
                        let a535 = v0;
                        let a536 = 8u32;
                        fixed_u32::bit_and(a535, a536)
                    };
                    let a538 = 0u32;
                    if equal(a537, a538) {
                        let a541 = {
                            let a539 = v0;
                            let a540 = 4u32;
                            fixed_u32::bit_and(a539, a540)
                        };
                        let a542 = 0u32;
                        if equal(a541, a542) {
                            let a545 = {
                                let a543 = v0;
                                let a544 = 2u32;
                                fixed_u32::bit_and(a543, a544)
                            };
                            let a546 = 0u32;
                            if equal(a545, a546) {
                                let a549 = {
                                    let a547 = v0;
                                    let a548 = 1u32;
                                    fixed_u32::bit_and(a547, a548)
                                };
                                let a550 = 0u32;
                                if equal(a549, a550) {
                                    Bytes::lit(&[0x20])
                                } else {
                                    Bytes::lit(&[0x21])
                                }
                            } else {
                                let a553 = {
                                    let a551 = v0;
                                    let a552 = 1u32;
                                    fixed_u32::bit_and(a551, a552)
                                };
                                let a554 = 0u32;
                                if equal(a553, a554) {
                                    Bytes::lit(&[0x22])
                                } else {
                                    Bytes::lit(&[0x23])
                                }
                            }
                        } else {
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
                                    Bytes::lit(&[0x24])
                                } else {
                                    Bytes::lit(&[0x25])
                                }
                            } else {
                                let a565 = {
                                    let a563 = v0;
                                    let a564 = 1u32;
                                    fixed_u32::bit_and(a563, a564)
                                };
                                let a566 = 0u32;
                                if equal(a565, a566) {
                                    Bytes::lit(&[0x26])
                                } else {
                                    Bytes::lit(&[0x27])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0x28])
                                } else {
                                    Bytes::lit(&[0x29])
                                }
                            } else {
                                let a581 = {
                                    let a579 = v0;
                                    let a580 = 1u32;
                                    fixed_u32::bit_and(a579, a580)
                                };
                                let a582 = 0u32;
                                if equal(a581, a582) {
                                    Bytes::lit(&[0x2a])
                                } else {
                                    Bytes::lit(&[0x2b])
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
                                    Bytes::lit(&[0x2c])
                                } else {
                                    Bytes::lit(&[0x2d])
                                }
                            } else {
                                let a593 = {
                                    let a591 = v0;
                                    let a592 = 1u32;
                                    fixed_u32::bit_and(a591, a592)
                                };
                                let a594 = 0u32;
                                if equal(a593, a594) {
                                    Bytes::lit(&[0x2e])
                                } else {
                                    Bytes::lit(&[0x2f])
                                }
                            }
                        }
                    }
                } else {
                    let a597 = {
                        let a595 = v0;
                        let a596 = 8u32;
                        fixed_u32::bit_and(a595, a596)
                    };
                    let a598 = 0u32;
                    if equal(a597, a598) {
                        let a601 = {
                            let a599 = v0;
                            let a600 = 4u32;
                            fixed_u32::bit_and(a599, a600)
                        };
                        let a602 = 0u32;
                        if equal(a601, a602) {
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
                                    Bytes::lit(&[0x30])
                                } else {
                                    Bytes::lit(&[0x31])
                                }
                            } else {
                                let a613 = {
                                    let a611 = v0;
                                    let a612 = 1u32;
                                    fixed_u32::bit_and(a611, a612)
                                };
                                let a614 = 0u32;
                                if equal(a613, a614) {
                                    Bytes::lit(&[0x32])
                                } else {
                                    Bytes::lit(&[0x33])
                                }
                            }
                        } else {
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
                                    Bytes::lit(&[0x34])
                                } else {
                                    Bytes::lit(&[0x35])
                                }
                            } else {
                                let a625 = {
                                    let a623 = v0;
                                    let a624 = 1u32;
                                    fixed_u32::bit_and(a623, a624)
                                };
                                let a626 = 0u32;
                                if equal(a625, a626) {
                                    Bytes::lit(&[0x36])
                                } else {
                                    Bytes::lit(&[0x37])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0x38])
                                } else {
                                    Bytes::lit(&[0x39])
                                }
                            } else {
                                let a641 = {
                                    let a639 = v0;
                                    let a640 = 1u32;
                                    fixed_u32::bit_and(a639, a640)
                                };
                                let a642 = 0u32;
                                if equal(a641, a642) {
                                    Bytes::lit(&[0x3a])
                                } else {
                                    Bytes::lit(&[0x3b])
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
                                    Bytes::lit(&[0x3c])
                                } else {
                                    Bytes::lit(&[0x3d])
                                }
                            } else {
                                let a653 = {
                                    let a651 = v0;
                                    let a652 = 1u32;
                                    fixed_u32::bit_and(a651, a652)
                                };
                                let a654 = 0u32;
                                if equal(a653, a654) {
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
            let a657 = {
                let a655 = v0;
                let a656 = 32u32;
                fixed_u32::bit_and(a655, a656)
            };
            let a658 = 0u32;
            if equal(a657, a658) {
                let a661 = {
                    let a659 = v0;
                    let a660 = 16u32;
                    fixed_u32::bit_and(a659, a660)
                };
                let a662 = 0u32;
                if equal(a661, a662) {
                    let a665 = {
                        let a663 = v0;
                        let a664 = 8u32;
                        fixed_u32::bit_and(a663, a664)
                    };
                    let a666 = 0u32;
                    if equal(a665, a666) {
                        let a669 = {
                            let a667 = v0;
                            let a668 = 4u32;
                            fixed_u32::bit_and(a667, a668)
                        };
                        let a670 = 0u32;
                        if equal(a669, a670) {
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
                                    Bytes::lit(&[0x40])
                                } else {
                                    Bytes::lit(&[0x41])
                                }
                            } else {
                                let a681 = {
                                    let a679 = v0;
                                    let a680 = 1u32;
                                    fixed_u32::bit_and(a679, a680)
                                };
                                let a682 = 0u32;
                                if equal(a681, a682) {
                                    Bytes::lit(&[0x42])
                                } else {
                                    Bytes::lit(&[0x43])
                                }
                            }
                        } else {
                            let a685 = {
                                let a683 = v0;
                                let a684 = 2u32;
                                fixed_u32::bit_and(a683, a684)
                            };
                            let a686 = 0u32;
                            if equal(a685, a686) {
                                let a689 = {
                                    let a687 = v0;
                                    let a688 = 1u32;
                                    fixed_u32::bit_and(a687, a688)
                                };
                                let a690 = 0u32;
                                if equal(a689, a690) {
                                    Bytes::lit(&[0x44])
                                } else {
                                    Bytes::lit(&[0x45])
                                }
                            } else {
                                let a693 = {
                                    let a691 = v0;
                                    let a692 = 1u32;
                                    fixed_u32::bit_and(a691, a692)
                                };
                                let a694 = 0u32;
                                if equal(a693, a694) {
                                    Bytes::lit(&[0x46])
                                } else {
                                    Bytes::lit(&[0x47])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0x48])
                                } else {
                                    Bytes::lit(&[0x49])
                                }
                            } else {
                                let a709 = {
                                    let a707 = v0;
                                    let a708 = 1u32;
                                    fixed_u32::bit_and(a707, a708)
                                };
                                let a710 = 0u32;
                                if equal(a709, a710) {
                                    Bytes::lit(&[0x4a])
                                } else {
                                    Bytes::lit(&[0x4b])
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
                                    Bytes::lit(&[0x4c])
                                } else {
                                    Bytes::lit(&[0x4d])
                                }
                            } else {
                                let a721 = {
                                    let a719 = v0;
                                    let a720 = 1u32;
                                    fixed_u32::bit_and(a719, a720)
                                };
                                let a722 = 0u32;
                                if equal(a721, a722) {
                                    Bytes::lit(&[0x4e])
                                } else {
                                    Bytes::lit(&[0x4f])
                                }
                            }
                        }
                    }
                } else {
                    let a725 = {
                        let a723 = v0;
                        let a724 = 8u32;
                        fixed_u32::bit_and(a723, a724)
                    };
                    let a726 = 0u32;
                    if equal(a725, a726) {
                        let a729 = {
                            let a727 = v0;
                            let a728 = 4u32;
                            fixed_u32::bit_and(a727, a728)
                        };
                        let a730 = 0u32;
                        if equal(a729, a730) {
                            let a733 = {
                                let a731 = v0;
                                let a732 = 2u32;
                                fixed_u32::bit_and(a731, a732)
                            };
                            let a734 = 0u32;
                            if equal(a733, a734) {
                                let a737 = {
                                    let a735 = v0;
                                    let a736 = 1u32;
                                    fixed_u32::bit_and(a735, a736)
                                };
                                let a738 = 0u32;
                                if equal(a737, a738) {
                                    Bytes::lit(&[0x50])
                                } else {
                                    Bytes::lit(&[0x51])
                                }
                            } else {
                                let a741 = {
                                    let a739 = v0;
                                    let a740 = 1u32;
                                    fixed_u32::bit_and(a739, a740)
                                };
                                let a742 = 0u32;
                                if equal(a741, a742) {
                                    Bytes::lit(&[0x52])
                                } else {
                                    Bytes::lit(&[0x53])
                                }
                            }
                        } else {
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
                                    Bytes::lit(&[0x54])
                                } else {
                                    Bytes::lit(&[0x55])
                                }
                            } else {
                                let a753 = {
                                    let a751 = v0;
                                    let a752 = 1u32;
                                    fixed_u32::bit_and(a751, a752)
                                };
                                let a754 = 0u32;
                                if equal(a753, a754) {
                                    Bytes::lit(&[0x56])
                                } else {
                                    Bytes::lit(&[0x57])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0x58])
                                } else {
                                    Bytes::lit(&[0x59])
                                }
                            } else {
                                let a769 = {
                                    let a767 = v0;
                                    let a768 = 1u32;
                                    fixed_u32::bit_and(a767, a768)
                                };
                                let a770 = 0u32;
                                if equal(a769, a770) {
                                    Bytes::lit(&[0x5a])
                                } else {
                                    Bytes::lit(&[0x5b])
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
                                    Bytes::lit(&[0x5c])
                                } else {
                                    Bytes::lit(&[0x5d])
                                }
                            } else {
                                let a781 = {
                                    let a779 = v0;
                                    let a780 = 1u32;
                                    fixed_u32::bit_and(a779, a780)
                                };
                                let a782 = 0u32;
                                if equal(a781, a782) {
                                    Bytes::lit(&[0x5e])
                                } else {
                                    Bytes::lit(&[0x5f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a785 = {
                    let a783 = v0;
                    let a784 = 16u32;
                    fixed_u32::bit_and(a783, a784)
                };
                let a786 = 0u32;
                if equal(a785, a786) {
                    let a789 = {
                        let a787 = v0;
                        let a788 = 8u32;
                        fixed_u32::bit_and(a787, a788)
                    };
                    let a790 = 0u32;
                    if equal(a789, a790) {
                        let a793 = {
                            let a791 = v0;
                            let a792 = 4u32;
                            fixed_u32::bit_and(a791, a792)
                        };
                        let a794 = 0u32;
                        if equal(a793, a794) {
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
                                    Bytes::lit(&[0x60])
                                } else {
                                    Bytes::lit(&[0x61])
                                }
                            } else {
                                let a805 = {
                                    let a803 = v0;
                                    let a804 = 1u32;
                                    fixed_u32::bit_and(a803, a804)
                                };
                                let a806 = 0u32;
                                if equal(a805, a806) {
                                    Bytes::lit(&[0x62])
                                } else {
                                    Bytes::lit(&[0x63])
                                }
                            }
                        } else {
                            let a809 = {
                                let a807 = v0;
                                let a808 = 2u32;
                                fixed_u32::bit_and(a807, a808)
                            };
                            let a810 = 0u32;
                            if equal(a809, a810) {
                                let a813 = {
                                    let a811 = v0;
                                    let a812 = 1u32;
                                    fixed_u32::bit_and(a811, a812)
                                };
                                let a814 = 0u32;
                                if equal(a813, a814) {
                                    Bytes::lit(&[0x64])
                                } else {
                                    Bytes::lit(&[0x65])
                                }
                            } else {
                                let a817 = {
                                    let a815 = v0;
                                    let a816 = 1u32;
                                    fixed_u32::bit_and(a815, a816)
                                };
                                let a818 = 0u32;
                                if equal(a817, a818) {
                                    Bytes::lit(&[0x66])
                                } else {
                                    Bytes::lit(&[0x67])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0x68])
                                } else {
                                    Bytes::lit(&[0x69])
                                }
                            } else {
                                let a833 = {
                                    let a831 = v0;
                                    let a832 = 1u32;
                                    fixed_u32::bit_and(a831, a832)
                                };
                                let a834 = 0u32;
                                if equal(a833, a834) {
                                    Bytes::lit(&[0x6a])
                                } else {
                                    Bytes::lit(&[0x6b])
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
                                    Bytes::lit(&[0x6c])
                                } else {
                                    Bytes::lit(&[0x6d])
                                }
                            } else {
                                let a845 = {
                                    let a843 = v0;
                                    let a844 = 1u32;
                                    fixed_u32::bit_and(a843, a844)
                                };
                                let a846 = 0u32;
                                if equal(a845, a846) {
                                    Bytes::lit(&[0x6e])
                                } else {
                                    Bytes::lit(&[0x6f])
                                }
                            }
                        }
                    }
                } else {
                    let a849 = {
                        let a847 = v0;
                        let a848 = 8u32;
                        fixed_u32::bit_and(a847, a848)
                    };
                    let a850 = 0u32;
                    if equal(a849, a850) {
                        let a853 = {
                            let a851 = v0;
                            let a852 = 4u32;
                            fixed_u32::bit_and(a851, a852)
                        };
                        let a854 = 0u32;
                        if equal(a853, a854) {
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
                                    Bytes::lit(&[0x70])
                                } else {
                                    Bytes::lit(&[0x71])
                                }
                            } else {
                                let a865 = {
                                    let a863 = v0;
                                    let a864 = 1u32;
                                    fixed_u32::bit_and(a863, a864)
                                };
                                let a866 = 0u32;
                                if equal(a865, a866) {
                                    Bytes::lit(&[0x72])
                                } else {
                                    Bytes::lit(&[0x73])
                                }
                            }
                        } else {
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
                                    Bytes::lit(&[0x74])
                                } else {
                                    Bytes::lit(&[0x75])
                                }
                            } else {
                                let a877 = {
                                    let a875 = v0;
                                    let a876 = 1u32;
                                    fixed_u32::bit_and(a875, a876)
                                };
                                let a878 = 0u32;
                                if equal(a877, a878) {
                                    Bytes::lit(&[0x76])
                                } else {
                                    Bytes::lit(&[0x77])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0x78])
                                } else {
                                    Bytes::lit(&[0x79])
                                }
                            } else {
                                let a893 = {
                                    let a891 = v0;
                                    let a892 = 1u32;
                                    fixed_u32::bit_and(a891, a892)
                                };
                                let a894 = 0u32;
                                if equal(a893, a894) {
                                    Bytes::lit(&[0x7a])
                                } else {
                                    Bytes::lit(&[0x7b])
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
                                    Bytes::lit(&[0x7c])
                                } else {
                                    Bytes::lit(&[0x7d])
                                }
                            } else {
                                let a905 = {
                                    let a903 = v0;
                                    let a904 = 1u32;
                                    fixed_u32::bit_and(a903, a904)
                                };
                                let a906 = 0u32;
                                if equal(a905, a906) {
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
        let a909 = {
            let a907 = v0;
            let a908 = 64u32;
            fixed_u32::bit_and(a907, a908)
        };
        let a910 = 0u32;
        if equal(a909, a910) {
            let a913 = {
                let a911 = v0;
                let a912 = 32u32;
                fixed_u32::bit_and(a911, a912)
            };
            let a914 = 0u32;
            if equal(a913, a914) {
                let a917 = {
                    let a915 = v0;
                    let a916 = 16u32;
                    fixed_u32::bit_and(a915, a916)
                };
                let a918 = 0u32;
                if equal(a917, a918) {
                    let a921 = {
                        let a919 = v0;
                        let a920 = 8u32;
                        fixed_u32::bit_and(a919, a920)
                    };
                    let a922 = 0u32;
                    if equal(a921, a922) {
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
                                    Bytes::lit(&[0x80])
                                } else {
                                    Bytes::lit(&[0x81])
                                }
                            } else {
                                let a937 = {
                                    let a935 = v0;
                                    let a936 = 1u32;
                                    fixed_u32::bit_and(a935, a936)
                                };
                                let a938 = 0u32;
                                if equal(a937, a938) {
                                    Bytes::lit(&[0x82])
                                } else {
                                    Bytes::lit(&[0x83])
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
                                    Bytes::lit(&[0x84])
                                } else {
                                    Bytes::lit(&[0x85])
                                }
                            } else {
                                let a949 = {
                                    let a947 = v0;
                                    let a948 = 1u32;
                                    fixed_u32::bit_and(a947, a948)
                                };
                                let a950 = 0u32;
                                if equal(a949, a950) {
                                    Bytes::lit(&[0x86])
                                } else {
                                    Bytes::lit(&[0x87])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0x88])
                                } else {
                                    Bytes::lit(&[0x89])
                                }
                            } else {
                                let a965 = {
                                    let a963 = v0;
                                    let a964 = 1u32;
                                    fixed_u32::bit_and(a963, a964)
                                };
                                let a966 = 0u32;
                                if equal(a965, a966) {
                                    Bytes::lit(&[0x8a])
                                } else {
                                    Bytes::lit(&[0x8b])
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
                                    Bytes::lit(&[0x8c])
                                } else {
                                    Bytes::lit(&[0x8d])
                                }
                            } else {
                                let a977 = {
                                    let a975 = v0;
                                    let a976 = 1u32;
                                    fixed_u32::bit_and(a975, a976)
                                };
                                let a978 = 0u32;
                                if equal(a977, a978) {
                                    Bytes::lit(&[0x8e])
                                } else {
                                    Bytes::lit(&[0x8f])
                                }
                            }
                        }
                    }
                } else {
                    let a981 = {
                        let a979 = v0;
                        let a980 = 8u32;
                        fixed_u32::bit_and(a979, a980)
                    };
                    let a982 = 0u32;
                    if equal(a981, a982) {
                        let a985 = {
                            let a983 = v0;
                            let a984 = 4u32;
                            fixed_u32::bit_and(a983, a984)
                        };
                        let a986 = 0u32;
                        if equal(a985, a986) {
                            let a989 = {
                                let a987 = v0;
                                let a988 = 2u32;
                                fixed_u32::bit_and(a987, a988)
                            };
                            let a990 = 0u32;
                            if equal(a989, a990) {
                                let a993 = {
                                    let a991 = v0;
                                    let a992 = 1u32;
                                    fixed_u32::bit_and(a991, a992)
                                };
                                let a994 = 0u32;
                                if equal(a993, a994) {
                                    Bytes::lit(&[0x90])
                                } else {
                                    Bytes::lit(&[0x91])
                                }
                            } else {
                                let a997 = {
                                    let a995 = v0;
                                    let a996 = 1u32;
                                    fixed_u32::bit_and(a995, a996)
                                };
                                let a998 = 0u32;
                                if equal(a997, a998) {
                                    Bytes::lit(&[0x92])
                                } else {
                                    Bytes::lit(&[0x93])
                                }
                            }
                        } else {
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
                                    Bytes::lit(&[0x94])
                                } else {
                                    Bytes::lit(&[0x95])
                                }
                            } else {
                                let a1009 = {
                                    let a1007 = v0;
                                    let a1008 = 1u32;
                                    fixed_u32::bit_and(a1007, a1008)
                                };
                                let a1010 = 0u32;
                                if equal(a1009, a1010) {
                                    Bytes::lit(&[0x96])
                                } else {
                                    Bytes::lit(&[0x97])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0x98])
                                } else {
                                    Bytes::lit(&[0x99])
                                }
                            } else {
                                let a1025 = {
                                    let a1023 = v0;
                                    let a1024 = 1u32;
                                    fixed_u32::bit_and(a1023, a1024)
                                };
                                let a1026 = 0u32;
                                if equal(a1025, a1026) {
                                    Bytes::lit(&[0x9a])
                                } else {
                                    Bytes::lit(&[0x9b])
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
                                    Bytes::lit(&[0x9c])
                                } else {
                                    Bytes::lit(&[0x9d])
                                }
                            } else {
                                let a1037 = {
                                    let a1035 = v0;
                                    let a1036 = 1u32;
                                    fixed_u32::bit_and(a1035, a1036)
                                };
                                let a1038 = 0u32;
                                if equal(a1037, a1038) {
                                    Bytes::lit(&[0x9e])
                                } else {
                                    Bytes::lit(&[0x9f])
                                }
                            }
                        }
                    }
                }
            } else {
                let a1041 = {
                    let a1039 = v0;
                    let a1040 = 16u32;
                    fixed_u32::bit_and(a1039, a1040)
                };
                let a1042 = 0u32;
                if equal(a1041, a1042) {
                    let a1045 = {
                        let a1043 = v0;
                        let a1044 = 8u32;
                        fixed_u32::bit_and(a1043, a1044)
                    };
                    let a1046 = 0u32;
                    if equal(a1045, a1046) {
                        let a1049 = {
                            let a1047 = v0;
                            let a1048 = 4u32;
                            fixed_u32::bit_and(a1047, a1048)
                        };
                        let a1050 = 0u32;
                        if equal(a1049, a1050) {
                            let a1053 = {
                                let a1051 = v0;
                                let a1052 = 2u32;
                                fixed_u32::bit_and(a1051, a1052)
                            };
                            let a1054 = 0u32;
                            if equal(a1053, a1054) {
                                let a1057 = {
                                    let a1055 = v0;
                                    let a1056 = 1u32;
                                    fixed_u32::bit_and(a1055, a1056)
                                };
                                let a1058 = 0u32;
                                if equal(a1057, a1058) {
                                    Bytes::lit(&[0xa0])
                                } else {
                                    Bytes::lit(&[0xa1])
                                }
                            } else {
                                let a1061 = {
                                    let a1059 = v0;
                                    let a1060 = 1u32;
                                    fixed_u32::bit_and(a1059, a1060)
                                };
                                let a1062 = 0u32;
                                if equal(a1061, a1062) {
                                    Bytes::lit(&[0xa2])
                                } else {
                                    Bytes::lit(&[0xa3])
                                }
                            }
                        } else {
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
                                    Bytes::lit(&[0xa4])
                                } else {
                                    Bytes::lit(&[0xa5])
                                }
                            } else {
                                let a1073 = {
                                    let a1071 = v0;
                                    let a1072 = 1u32;
                                    fixed_u32::bit_and(a1071, a1072)
                                };
                                let a1074 = 0u32;
                                if equal(a1073, a1074) {
                                    Bytes::lit(&[0xa6])
                                } else {
                                    Bytes::lit(&[0xa7])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0xa8])
                                } else {
                                    Bytes::lit(&[0xa9])
                                }
                            } else {
                                let a1089 = {
                                    let a1087 = v0;
                                    let a1088 = 1u32;
                                    fixed_u32::bit_and(a1087, a1088)
                                };
                                let a1090 = 0u32;
                                if equal(a1089, a1090) {
                                    Bytes::lit(&[0xaa])
                                } else {
                                    Bytes::lit(&[0xab])
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
                                    Bytes::lit(&[0xac])
                                } else {
                                    Bytes::lit(&[0xad])
                                }
                            } else {
                                let a1101 = {
                                    let a1099 = v0;
                                    let a1100 = 1u32;
                                    fixed_u32::bit_and(a1099, a1100)
                                };
                                let a1102 = 0u32;
                                if equal(a1101, a1102) {
                                    Bytes::lit(&[0xae])
                                } else {
                                    Bytes::lit(&[0xaf])
                                }
                            }
                        }
                    }
                } else {
                    let a1105 = {
                        let a1103 = v0;
                        let a1104 = 8u32;
                        fixed_u32::bit_and(a1103, a1104)
                    };
                    let a1106 = 0u32;
                    if equal(a1105, a1106) {
                        let a1109 = {
                            let a1107 = v0;
                            let a1108 = 4u32;
                            fixed_u32::bit_and(a1107, a1108)
                        };
                        let a1110 = 0u32;
                        if equal(a1109, a1110) {
                            let a1113 = {
                                let a1111 = v0;
                                let a1112 = 2u32;
                                fixed_u32::bit_and(a1111, a1112)
                            };
                            let a1114 = 0u32;
                            if equal(a1113, a1114) {
                                let a1117 = {
                                    let a1115 = v0;
                                    let a1116 = 1u32;
                                    fixed_u32::bit_and(a1115, a1116)
                                };
                                let a1118 = 0u32;
                                if equal(a1117, a1118) {
                                    Bytes::lit(&[0xb0])
                                } else {
                                    Bytes::lit(&[0xb1])
                                }
                            } else {
                                let a1121 = {
                                    let a1119 = v0;
                                    let a1120 = 1u32;
                                    fixed_u32::bit_and(a1119, a1120)
                                };
                                let a1122 = 0u32;
                                if equal(a1121, a1122) {
                                    Bytes::lit(&[0xb2])
                                } else {
                                    Bytes::lit(&[0xb3])
                                }
                            }
                        } else {
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
                                    Bytes::lit(&[0xb4])
                                } else {
                                    Bytes::lit(&[0xb5])
                                }
                            } else {
                                let a1133 = {
                                    let a1131 = v0;
                                    let a1132 = 1u32;
                                    fixed_u32::bit_and(a1131, a1132)
                                };
                                let a1134 = 0u32;
                                if equal(a1133, a1134) {
                                    Bytes::lit(&[0xb6])
                                } else {
                                    Bytes::lit(&[0xb7])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0xb8])
                                } else {
                                    Bytes::lit(&[0xb9])
                                }
                            } else {
                                let a1149 = {
                                    let a1147 = v0;
                                    let a1148 = 1u32;
                                    fixed_u32::bit_and(a1147, a1148)
                                };
                                let a1150 = 0u32;
                                if equal(a1149, a1150) {
                                    Bytes::lit(&[0xba])
                                } else {
                                    Bytes::lit(&[0xbb])
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
                                    Bytes::lit(&[0xbc])
                                } else {
                                    Bytes::lit(&[0xbd])
                                }
                            } else {
                                let a1161 = {
                                    let a1159 = v0;
                                    let a1160 = 1u32;
                                    fixed_u32::bit_and(a1159, a1160)
                                };
                                let a1162 = 0u32;
                                if equal(a1161, a1162) {
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
            let a1165 = {
                let a1163 = v0;
                let a1164 = 32u32;
                fixed_u32::bit_and(a1163, a1164)
            };
            let a1166 = 0u32;
            if equal(a1165, a1166) {
                let a1169 = {
                    let a1167 = v0;
                    let a1168 = 16u32;
                    fixed_u32::bit_and(a1167, a1168)
                };
                let a1170 = 0u32;
                if equal(a1169, a1170) {
                    let a1173 = {
                        let a1171 = v0;
                        let a1172 = 8u32;
                        fixed_u32::bit_and(a1171, a1172)
                    };
                    let a1174 = 0u32;
                    if equal(a1173, a1174) {
                        let a1177 = {
                            let a1175 = v0;
                            let a1176 = 4u32;
                            fixed_u32::bit_and(a1175, a1176)
                        };
                        let a1178 = 0u32;
                        if equal(a1177, a1178) {
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
                                    Bytes::lit(&[0xc0])
                                } else {
                                    Bytes::lit(&[0xc1])
                                }
                            } else {
                                let a1189 = {
                                    let a1187 = v0;
                                    let a1188 = 1u32;
                                    fixed_u32::bit_and(a1187, a1188)
                                };
                                let a1190 = 0u32;
                                if equal(a1189, a1190) {
                                    Bytes::lit(&[0xc2])
                                } else {
                                    Bytes::lit(&[0xc3])
                                }
                            }
                        } else {
                            let a1193 = {
                                let a1191 = v0;
                                let a1192 = 2u32;
                                fixed_u32::bit_and(a1191, a1192)
                            };
                            let a1194 = 0u32;
                            if equal(a1193, a1194) {
                                let a1197 = {
                                    let a1195 = v0;
                                    let a1196 = 1u32;
                                    fixed_u32::bit_and(a1195, a1196)
                                };
                                let a1198 = 0u32;
                                if equal(a1197, a1198) {
                                    Bytes::lit(&[0xc4])
                                } else {
                                    Bytes::lit(&[0xc5])
                                }
                            } else {
                                let a1201 = {
                                    let a1199 = v0;
                                    let a1200 = 1u32;
                                    fixed_u32::bit_and(a1199, a1200)
                                };
                                let a1202 = 0u32;
                                if equal(a1201, a1202) {
                                    Bytes::lit(&[0xc6])
                                } else {
                                    Bytes::lit(&[0xc7])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0xc8])
                                } else {
                                    Bytes::lit(&[0xc9])
                                }
                            } else {
                                let a1217 = {
                                    let a1215 = v0;
                                    let a1216 = 1u32;
                                    fixed_u32::bit_and(a1215, a1216)
                                };
                                let a1218 = 0u32;
                                if equal(a1217, a1218) {
                                    Bytes::lit(&[0xca])
                                } else {
                                    Bytes::lit(&[0xcb])
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
                                    Bytes::lit(&[0xcc])
                                } else {
                                    Bytes::lit(&[0xcd])
                                }
                            } else {
                                let a1229 = {
                                    let a1227 = v0;
                                    let a1228 = 1u32;
                                    fixed_u32::bit_and(a1227, a1228)
                                };
                                let a1230 = 0u32;
                                if equal(a1229, a1230) {
                                    Bytes::lit(&[0xce])
                                } else {
                                    Bytes::lit(&[0xcf])
                                }
                            }
                        }
                    }
                } else {
                    let a1233 = {
                        let a1231 = v0;
                        let a1232 = 8u32;
                        fixed_u32::bit_and(a1231, a1232)
                    };
                    let a1234 = 0u32;
                    if equal(a1233, a1234) {
                        let a1237 = {
                            let a1235 = v0;
                            let a1236 = 4u32;
                            fixed_u32::bit_and(a1235, a1236)
                        };
                        let a1238 = 0u32;
                        if equal(a1237, a1238) {
                            let a1241 = {
                                let a1239 = v0;
                                let a1240 = 2u32;
                                fixed_u32::bit_and(a1239, a1240)
                            };
                            let a1242 = 0u32;
                            if equal(a1241, a1242) {
                                let a1245 = {
                                    let a1243 = v0;
                                    let a1244 = 1u32;
                                    fixed_u32::bit_and(a1243, a1244)
                                };
                                let a1246 = 0u32;
                                if equal(a1245, a1246) {
                                    Bytes::lit(&[0xd0])
                                } else {
                                    Bytes::lit(&[0xd1])
                                }
                            } else {
                                let a1249 = {
                                    let a1247 = v0;
                                    let a1248 = 1u32;
                                    fixed_u32::bit_and(a1247, a1248)
                                };
                                let a1250 = 0u32;
                                if equal(a1249, a1250) {
                                    Bytes::lit(&[0xd2])
                                } else {
                                    Bytes::lit(&[0xd3])
                                }
                            }
                        } else {
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
                                    Bytes::lit(&[0xd4])
                                } else {
                                    Bytes::lit(&[0xd5])
                                }
                            } else {
                                let a1261 = {
                                    let a1259 = v0;
                                    let a1260 = 1u32;
                                    fixed_u32::bit_and(a1259, a1260)
                                };
                                let a1262 = 0u32;
                                if equal(a1261, a1262) {
                                    Bytes::lit(&[0xd6])
                                } else {
                                    Bytes::lit(&[0xd7])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0xd8])
                                } else {
                                    Bytes::lit(&[0xd9])
                                }
                            } else {
                                let a1277 = {
                                    let a1275 = v0;
                                    let a1276 = 1u32;
                                    fixed_u32::bit_and(a1275, a1276)
                                };
                                let a1278 = 0u32;
                                if equal(a1277, a1278) {
                                    Bytes::lit(&[0xda])
                                } else {
                                    Bytes::lit(&[0xdb])
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
                                    Bytes::lit(&[0xdc])
                                } else {
                                    Bytes::lit(&[0xdd])
                                }
                            } else {
                                let a1289 = {
                                    let a1287 = v0;
                                    let a1288 = 1u32;
                                    fixed_u32::bit_and(a1287, a1288)
                                };
                                let a1290 = 0u32;
                                if equal(a1289, a1290) {
                                    Bytes::lit(&[0xde])
                                } else {
                                    Bytes::lit(&[0xdf])
                                }
                            }
                        }
                    }
                }
            } else {
                let a1293 = {
                    let a1291 = v0;
                    let a1292 = 16u32;
                    fixed_u32::bit_and(a1291, a1292)
                };
                let a1294 = 0u32;
                if equal(a1293, a1294) {
                    let a1297 = {
                        let a1295 = v0;
                        let a1296 = 8u32;
                        fixed_u32::bit_and(a1295, a1296)
                    };
                    let a1298 = 0u32;
                    if equal(a1297, a1298) {
                        let a1301 = {
                            let a1299 = v0;
                            let a1300 = 4u32;
                            fixed_u32::bit_and(a1299, a1300)
                        };
                        let a1302 = 0u32;
                        if equal(a1301, a1302) {
                            let a1305 = {
                                let a1303 = v0;
                                let a1304 = 2u32;
                                fixed_u32::bit_and(a1303, a1304)
                            };
                            let a1306 = 0u32;
                            if equal(a1305, a1306) {
                                let a1309 = {
                                    let a1307 = v0;
                                    let a1308 = 1u32;
                                    fixed_u32::bit_and(a1307, a1308)
                                };
                                let a1310 = 0u32;
                                if equal(a1309, a1310) {
                                    Bytes::lit(&[0xe0])
                                } else {
                                    Bytes::lit(&[0xe1])
                                }
                            } else {
                                let a1313 = {
                                    let a1311 = v0;
                                    let a1312 = 1u32;
                                    fixed_u32::bit_and(a1311, a1312)
                                };
                                let a1314 = 0u32;
                                if equal(a1313, a1314) {
                                    Bytes::lit(&[0xe2])
                                } else {
                                    Bytes::lit(&[0xe3])
                                }
                            }
                        } else {
                            let a1317 = {
                                let a1315 = v0;
                                let a1316 = 2u32;
                                fixed_u32::bit_and(a1315, a1316)
                            };
                            let a1318 = 0u32;
                            if equal(a1317, a1318) {
                                let a1321 = {
                                    let a1319 = v0;
                                    let a1320 = 1u32;
                                    fixed_u32::bit_and(a1319, a1320)
                                };
                                let a1322 = 0u32;
                                if equal(a1321, a1322) {
                                    Bytes::lit(&[0xe4])
                                } else {
                                    Bytes::lit(&[0xe5])
                                }
                            } else {
                                let a1325 = {
                                    let a1323 = v0;
                                    let a1324 = 1u32;
                                    fixed_u32::bit_and(a1323, a1324)
                                };
                                let a1326 = 0u32;
                                if equal(a1325, a1326) {
                                    Bytes::lit(&[0xe6])
                                } else {
                                    Bytes::lit(&[0xe7])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0xe8])
                                } else {
                                    Bytes::lit(&[0xe9])
                                }
                            } else {
                                let a1341 = {
                                    let a1339 = v0;
                                    let a1340 = 1u32;
                                    fixed_u32::bit_and(a1339, a1340)
                                };
                                let a1342 = 0u32;
                                if equal(a1341, a1342) {
                                    Bytes::lit(&[0xea])
                                } else {
                                    Bytes::lit(&[0xeb])
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
                                    Bytes::lit(&[0xec])
                                } else {
                                    Bytes::lit(&[0xed])
                                }
                            } else {
                                let a1353 = {
                                    let a1351 = v0;
                                    let a1352 = 1u32;
                                    fixed_u32::bit_and(a1351, a1352)
                                };
                                let a1354 = 0u32;
                                if equal(a1353, a1354) {
                                    Bytes::lit(&[0xee])
                                } else {
                                    Bytes::lit(&[0xef])
                                }
                            }
                        }
                    }
                } else {
                    let a1357 = {
                        let a1355 = v0;
                        let a1356 = 8u32;
                        fixed_u32::bit_and(a1355, a1356)
                    };
                    let a1358 = 0u32;
                    if equal(a1357, a1358) {
                        let a1361 = {
                            let a1359 = v0;
                            let a1360 = 4u32;
                            fixed_u32::bit_and(a1359, a1360)
                        };
                        let a1362 = 0u32;
                        if equal(a1361, a1362) {
                            let a1365 = {
                                let a1363 = v0;
                                let a1364 = 2u32;
                                fixed_u32::bit_and(a1363, a1364)
                            };
                            let a1366 = 0u32;
                            if equal(a1365, a1366) {
                                let a1369 = {
                                    let a1367 = v0;
                                    let a1368 = 1u32;
                                    fixed_u32::bit_and(a1367, a1368)
                                };
                                let a1370 = 0u32;
                                if equal(a1369, a1370) {
                                    Bytes::lit(&[0xf0])
                                } else {
                                    Bytes::lit(&[0xf1])
                                }
                            } else {
                                let a1373 = {
                                    let a1371 = v0;
                                    let a1372 = 1u32;
                                    fixed_u32::bit_and(a1371, a1372)
                                };
                                let a1374 = 0u32;
                                if equal(a1373, a1374) {
                                    Bytes::lit(&[0xf2])
                                } else {
                                    Bytes::lit(&[0xf3])
                                }
                            }
                        } else {
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
                                    Bytes::lit(&[0xf4])
                                } else {
                                    Bytes::lit(&[0xf5])
                                }
                            } else {
                                let a1385 = {
                                    let a1383 = v0;
                                    let a1384 = 1u32;
                                    fixed_u32::bit_and(a1383, a1384)
                                };
                                let a1386 = 0u32;
                                if equal(a1385, a1386) {
                                    Bytes::lit(&[0xf6])
                                } else {
                                    Bytes::lit(&[0xf7])
                                }
                            }
                        }
                    } else {
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
                                    Bytes::lit(&[0xf8])
                                } else {
                                    Bytes::lit(&[0xf9])
                                }
                            } else {
                                let a1401 = {
                                    let a1399 = v0;
                                    let a1400 = 1u32;
                                    fixed_u32::bit_and(a1399, a1400)
                                };
                                let a1402 = 0u32;
                                if equal(a1401, a1402) {
                                    Bytes::lit(&[0xfa])
                                } else {
                                    Bytes::lit(&[0xfb])
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
                                    Bytes::lit(&[0xfc])
                                } else {
                                    Bytes::lit(&[0xfd])
                                }
                            } else {
                                let a1413 = {
                                    let a1411 = v0;
                                    let a1412 = 1u32;
                                    fixed_u32::bit_and(a1411, a1412)
                                };
                                let a1414 = 0u32;
                                if equal(a1413, a1414) {
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

pub fn f55(v0: u32, v1: u32) -> u32 {
    let m1417 = {
        let a1415 = v0;
        let a1416 = v1;
        fixed_u32::shift_right(a1415, a1416)
    };
    match m1417 {
        None => {
            0u32
        }
        Some(v2) => {
            v2
        }
    }
}

pub fn f56(v0: Adt5, v1: Adt5) -> Adt5 {
    let v2: Adt6 = f64(match v0.clone() {
        Adt5::C0(_, _, _, h1418, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1418
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, h1419, _, _, _, _, _, _, _, _, _, _, _) => {
            h1419
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, h1420, _, _, _, _, _, _) => {
            h1420
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1421, _) => {
            h1421
        }
    }, match v1.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1422, _) => {
            h1422
        }
    }, match v1 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1423) => {
            h1423
        }
    });
    Adt5::C0(match v0.clone() {
        Adt5::C0(h1424, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1424
        }
    }, match v0.clone() {
        Adt5::C0(_, h1425, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1425
        }
    }, match v0.clone() {
        Adt5::C0(_, _, h1426, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1426
        }
    }, match v2.clone() {
        Adt6::C0(h1427, _, _, _) => {
            h1427
        }
    }, match v2.clone() {
        Adt6::C0(_, h1428, _, _) => {
            h1428
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, h1429, _, _, _, _, _, _, _, _, _, _) => {
            h1429
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, h1430, _, _, _, _, _, _, _, _, _) => {
            h1430
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, h1431, _, _, _, _, _, _, _, _) => {
            h1431
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, h1432, _, _, _, _, _, _, _) => {
            h1432
        }
    }, match v2.clone() {
        Adt6::C0(_, _, h1433, _) => {
            h1433
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, h1434, _, _, _, _, _) => {
            h1434
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h1435, _, _, _, _) => {
            h1435
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1436, _, _, _) => {
            h1436
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1437, _, _) => {
            h1437
        }
    }, match v2 {
        Adt6::C0(_, _, _, h1438) => {
            h1438
        }
    }, match v0 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1439) => {
            h1439
        }
    })
}

pub fn f57(v0: Adt5, v1: Adt5) -> Adt5 {
    let v2: Adt6 = f64(match v0.clone() {
        Adt5::C0(_, _, h1440, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1440
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, h1441, _, _, _, _, _, _, _, _) => {
            h1441
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, h1442, _, _, _, _, _, _, _) => {
            h1442
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1443, _, _) => {
            h1443
        }
    }, match v1.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1444, _, _, _) => {
            h1444
        }
    }, match v1 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1445, _, _) => {
            h1445
        }
    });
    Adt5::C0(match v0.clone() {
        Adt5::C0(h1446, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1446
        }
    }, match v0.clone() {
        Adt5::C0(_, h1447, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1447
        }
    }, match v2.clone() {
        Adt6::C0(h1448, _, _, _) => {
            h1448
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, h1449, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1449
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, h1450, _, _, _, _, _, _, _, _, _, _, _) => {
            h1450
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, h1451, _, _, _, _, _, _, _, _, _, _) => {
            h1451
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, h1452, _, _, _, _, _, _, _, _, _) => {
            h1452
        }
    }, match v2.clone() {
        Adt6::C0(_, h1453, _, _) => {
            h1453
        }
    }, match v2.clone() {
        Adt6::C0(_, _, h1454, _) => {
            h1454
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, h1455, _, _, _, _, _, _) => {
            h1455
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, h1456, _, _, _, _, _) => {
            h1456
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h1457, _, _, _, _) => {
            h1457
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1458, _, _, _) => {
            h1458
        }
    }, match v2 {
        Adt6::C0(_, _, _, h1459) => {
            h1459
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1460, _) => {
            h1460
        }
    }, match v0 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1461) => {
            h1461
        }
    })
}

pub fn f58(v0: Adt5, v1: Adt5) -> Adt5 {
    let v2: Adt6 = f64(match v0.clone() {
        Adt5::C0(_, h1462, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1462
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, h1463, _, _, _, _, _, _, _, _, _) => {
            h1463
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h1464, _, _, _, _) => {
            h1464
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1465, _, _, _) => {
            h1465
        }
    }, match v1.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, h1466, _, _, _, _, _) => {
            h1466
        }
    }, match v1 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h1467, _, _, _, _) => {
            h1467
        }
    });
    Adt5::C0(match v0.clone() {
        Adt5::C0(h1468, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1468
        }
    }, match v2.clone() {
        Adt6::C0(h1469, _, _, _) => {
            h1469
        }
    }, match v0.clone() {
        Adt5::C0(_, _, h1470, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1470
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, h1471, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1471
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, h1472, _, _, _, _, _, _, _, _, _, _, _) => {
            h1472
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, h1473, _, _, _, _, _, _, _, _, _, _) => {
            h1473
        }
    }, match v2.clone() {
        Adt6::C0(_, h1474, _, _) => {
            h1474
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, h1475, _, _, _, _, _, _, _, _) => {
            h1475
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, h1476, _, _, _, _, _, _, _) => {
            h1476
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, h1477, _, _, _, _, _, _) => {
            h1477
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, h1478, _, _, _, _, _) => {
            h1478
        }
    }, match v2.clone() {
        Adt6::C0(_, _, h1479, _) => {
            h1479
        }
    }, match v2 {
        Adt6::C0(_, _, _, h1480) => {
            h1480
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1481, _, _) => {
            h1481
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1482, _) => {
            h1482
        }
    }, match v0 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1483) => {
            h1483
        }
    })
}

pub fn f59(v0: Adt5, v1: Adt5) -> Adt5 {
    let v2: Adt6 = f64(match v0.clone() {
        Adt5::C0(h1484, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1484
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, h1485, _, _, _, _, _, _, _, _, _, _) => {
            h1485
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, h1486, _, _, _, _, _) => {
            h1486
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1487) => {
            h1487
        }
    }, match v1.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, h1488, _, _, _, _, _, _, _) => {
            h1488
        }
    }, match v1 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, h1489, _, _, _, _, _, _) => {
            h1489
        }
    });
    Adt5::C0(match v2.clone() {
        Adt6::C0(h1490, _, _, _) => {
            h1490
        }
    }, match v0.clone() {
        Adt5::C0(_, h1491, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1491
        }
    }, match v0.clone() {
        Adt5::C0(_, _, h1492, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1492
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, h1493, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1493
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, h1494, _, _, _, _, _, _, _, _, _, _, _) => {
            h1494
        }
    }, match v2.clone() {
        Adt6::C0(_, h1495, _, _) => {
            h1495
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, h1496, _, _, _, _, _, _, _, _, _) => {
            h1496
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, h1497, _, _, _, _, _, _, _, _) => {
            h1497
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, h1498, _, _, _, _, _, _, _) => {
            h1498
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, h1499, _, _, _, _, _, _) => {
            h1499
        }
    }, match v2.clone() {
        Adt6::C0(_, _, h1500, _) => {
            h1500
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h1501, _, _, _, _) => {
            h1501
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1502, _, _, _) => {
            h1502
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1503, _, _) => {
            h1503
        }
    }, match v0 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1504, _) => {
            h1504
        }
    }, match v2 {
        Adt6::C0(_, _, _, h1505) => {
            h1505
        }
    })
}

pub fn f60(v0: Adt5, v1: Adt5) -> Adt5 {
    let v2: Adt6 = f64(match v0.clone() {
        Adt5::C0(_, _, _, h1506, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1506
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, h1507, _, _, _, _, _, _, _, _) => {
            h1507
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h1508, _, _, _, _) => {
            h1508
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1509) => {
            h1509
        }
    }, match v1.clone() {
        Adt5::C0(_, _, _, _, _, _, h1510, _, _, _, _, _, _, _, _, _) => {
            h1510
        }
    }, match v1 {
        Adt5::C0(_, _, _, _, _, _, _, h1511, _, _, _, _, _, _, _, _) => {
            h1511
        }
    });
    Adt5::C0(match v0.clone() {
        Adt5::C0(h1512, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1512
        }
    }, match v0.clone() {
        Adt5::C0(_, h1513, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1513
        }
    }, match v0.clone() {
        Adt5::C0(_, _, h1514, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1514
        }
    }, match v2.clone() {
        Adt6::C0(h1515, _, _, _) => {
            h1515
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, h1516, _, _, _, _, _, _, _, _, _, _, _) => {
            h1516
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, h1517, _, _, _, _, _, _, _, _, _, _) => {
            h1517
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, h1518, _, _, _, _, _, _, _, _, _) => {
            h1518
        }
    }, match v2.clone() {
        Adt6::C0(_, h1519, _, _) => {
            h1519
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, h1520, _, _, _, _, _, _, _) => {
            h1520
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, h1521, _, _, _, _, _, _) => {
            h1521
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, h1522, _, _, _, _, _) => {
            h1522
        }
    }, match v2.clone() {
        Adt6::C0(_, _, h1523, _) => {
            h1523
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1524, _, _, _) => {
            h1524
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1525, _, _) => {
            h1525
        }
    }, match v0 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1526, _) => {
            h1526
        }
    }, match v2 {
        Adt6::C0(_, _, _, h1527) => {
            h1527
        }
    })
}

pub fn f61(v0: Adt5, v1: Adt5) -> Adt5 {
    let v2: Adt6 = f64(match v0.clone() {
        Adt5::C0(_, _, h1528, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1528
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, h1529, _, _, _, _, _, _, _, _, _) => {
            h1529
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, h1530, _, _, _, _, _) => {
            h1530
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1531, _) => {
            h1531
        }
    }, match v1.clone() {
        Adt5::C0(_, _, _, _, h1532, _, _, _, _, _, _, _, _, _, _, _) => {
            h1532
        }
    }, match v1 {
        Adt5::C0(_, _, _, _, _, h1533, _, _, _, _, _, _, _, _, _, _) => {
            h1533
        }
    });
    Adt5::C0(match v0.clone() {
        Adt5::C0(h1534, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1534
        }
    }, match v0.clone() {
        Adt5::C0(_, h1535, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1535
        }
    }, match v2.clone() {
        Adt6::C0(h1536, _, _, _) => {
            h1536
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, h1537, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1537
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, h1538, _, _, _, _, _, _, _, _, _, _, _) => {
            h1538
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, h1539, _, _, _, _, _, _, _, _, _, _) => {
            h1539
        }
    }, match v2.clone() {
        Adt6::C0(_, h1540, _, _) => {
            h1540
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, h1541, _, _, _, _, _, _, _, _) => {
            h1541
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, h1542, _, _, _, _, _, _, _) => {
            h1542
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, h1543, _, _, _, _, _, _) => {
            h1543
        }
    }, match v2.clone() {
        Adt6::C0(_, _, h1544, _) => {
            h1544
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h1545, _, _, _, _) => {
            h1545
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1546, _, _, _) => {
            h1546
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1547, _, _) => {
            h1547
        }
    }, match v2 {
        Adt6::C0(_, _, _, h1548) => {
            h1548
        }
    }, match v0 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1549) => {
            h1549
        }
    })
}

pub fn f62(v0: Adt5, v1: Adt5) -> Adt5 {
    let v2: Adt6 = f64(match v0.clone() {
        Adt5::C0(_, h1550, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1550
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, h1551, _, _, _, _, _, _, _, _, _, _) => {
            h1551
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, h1552, _, _, _, _, _, _) => {
            h1552
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1553, _, _) => {
            h1553
        }
    }, match v1.clone() {
        Adt5::C0(_, _, h1554, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1554
        }
    }, match v1 {
        Adt5::C0(_, _, _, h1555, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1555
        }
    });
    Adt5::C0(match v0.clone() {
        Adt5::C0(h1556, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1556
        }
    }, match v2.clone() {
        Adt6::C0(h1557, _, _, _) => {
            h1557
        }
    }, match v0.clone() {
        Adt5::C0(_, _, h1558, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1558
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, h1559, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1559
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, h1560, _, _, _, _, _, _, _, _, _, _, _) => {
            h1560
        }
    }, match v2.clone() {
        Adt6::C0(_, h1561, _, _) => {
            h1561
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, h1562, _, _, _, _, _, _, _, _, _) => {
            h1562
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, h1563, _, _, _, _, _, _, _, _) => {
            h1563
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, h1564, _, _, _, _, _, _, _) => {
            h1564
        }
    }, match v2.clone() {
        Adt6::C0(_, _, h1565, _) => {
            h1565
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, h1566, _, _, _, _, _) => {
            h1566
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h1567, _, _, _, _) => {
            h1567
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1568, _, _, _) => {
            h1568
        }
    }, match v2 {
        Adt6::C0(_, _, _, h1569) => {
            h1569
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1570, _) => {
            h1570
        }
    }, match v0 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1571) => {
            h1571
        }
    })
}

pub fn f63(v0: Adt5, v1: Adt5) -> Adt5 {
    let v2: Adt6 = f64(match v0.clone() {
        Adt5::C0(h1572, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1572
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, h1573, _, _, _, _, _, _, _, _, _, _, _) => {
            h1573
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, h1574, _, _, _, _, _, _, _) => {
            h1574
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, h1575, _, _, _) => {
            h1575
        }
    }, match v1.clone() {
        Adt5::C0(h1576, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1576
        }
    }, match v1 {
        Adt5::C0(_, h1577, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1577
        }
    });
    Adt5::C0(match v2.clone() {
        Adt6::C0(h1578, _, _, _) => {
            h1578
        }
    }, match v0.clone() {
        Adt5::C0(_, h1579, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1579
        }
    }, match v0.clone() {
        Adt5::C0(_, _, h1580, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1580
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, h1581, _, _, _, _, _, _, _, _, _, _, _, _) => {
            h1581
        }
    }, match v2.clone() {
        Adt6::C0(_, h1582, _, _) => {
            h1582
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, h1583, _, _, _, _, _, _, _, _, _, _) => {
            h1583
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, h1584, _, _, _, _, _, _, _, _, _) => {
            h1584
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, h1585, _, _, _, _, _, _, _, _) => {
            h1585
        }
    }, match v2.clone() {
        Adt6::C0(_, _, h1586, _) => {
            h1586
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, h1587, _, _, _, _, _, _) => {
            h1587
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, h1588, _, _, _, _, _) => {
            h1588
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, h1589, _, _, _, _) => {
            h1589
        }
    }, match v2 {
        Adt6::C0(_, _, _, h1590) => {
            h1590
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, h1591, _, _) => {
            h1591
        }
    }, match v0.clone() {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, h1592, _) => {
            h1592
        }
    }, match v0 {
        Adt5::C0(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, h1593) => {
            h1593
        }
    })
}

pub fn f64(v0: u32, v1: u32, v2: u32, v3: u32, v4: u32, v5: u32) -> Adt6 {
    let v6: u32 = f65(f65(v0, v1), v4);
    let v7: u32 = f66(f44(v3, v6), 16u32, 16u32);
    let v8: u32 = f65(v2, v7);
    let v9: u32 = f66(f44(v1, v8), 12u32, 20u32);
    let v10: u32 = f65(f65(v6, v9), v5);
    let v11: u32 = f66(f44(v7, v10), 8u32, 24u32);
    let v12: u32 = f65(v8, v11);
    let v13: u32 = f66(f44(v9, v12), 7u32, 25u32);
    Adt6::C0(v10, v13, v12, v11)
}

pub fn f65(v0: u32, v1: u32) -> u32 {
    let m1596 = {
        let a1594 = f67(v0);
        let a1595 = f67(v1);
        fixed_u64::checked_add(a1594, a1595)
    };
    match m1596 {
        None => {
            0u32
        }
        Some(v2) => {
            f68(v2)
        }
    }
}

pub fn f66(v0: u32, v1: u32, v2: u32) -> u32 {
    let a1597 = f55(v0, v1);
    let a1598 = f51(v0, v2);
    fixed_u32::bit_or(a1597, a1598)
}

pub fn f67(v0: u32) -> u64 {
    let m1600 = {
        let a1599 = v0;
        fixed_u64::convert(i128::from(a1599))
    };
    match m1600 {
        None => {
            0u64
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn f68(v0: u64) -> u32 {
    let m1604 = {
        let a1603 = {
            let a1601 = v0;
            let a1602 = 4294967295u64;
            fixed_u64::bit_and(a1601, a1602)
        };
        fixed_u32::convert(i128::from(a1603))
    };
    match m1604 {
        None => {
            0u32
        }
        Some(v1) => {
            v1
        }
    }
}

pub fn run(p0: Bytes, p1: Bytes, p2: u64) -> R<Result<Bytes, Adt0>> {
    f0(p0, p1, p2)
}
