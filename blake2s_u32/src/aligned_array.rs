use core::fmt;
use core::hash::Hash;
use core::ops::{Deref, DerefMut, Index, IndexMut};
use core::slice::SliceIndex;

#[repr(C)]
pub struct AlignedArray<T, A, const N: usize> {
    _aligner: [A; 0],
    data: [T; N],
}

#[repr(C)]
pub struct AlignedSlice<T, A> {
    _aligner: [A; 0],
    data: [T],
}

#[derive(Clone, Copy)]
#[repr(align(64))]
pub struct A64;

pub type AlignedArray64<T, const N: usize> = AlignedArray<T, A64, N>;
pub type AlignedSlice64<T> = AlignedSlice<T, A64>;

impl<T, A, const N: usize> AlignedArray<T, A, N> {
    pub const fn new(data: [T; N]) -> Self {
        Self { _aligner: [], data }
    }

    pub const fn as_array(&self) -> &[T; N] {
        &self.data
    }

    pub fn as_mut_array(&mut self) -> &mut [T; N] {
        &mut self.data
    }

    pub const fn as_slice(&self) -> &[T] {
        &self.data
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.data
    }

    pub const fn as_ptr(&self) -> *const T {
        self.data.as_ptr()
    }

    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.data.as_mut_ptr()
    }

    pub const fn len(&self) -> usize {
        N
    }

    pub const fn is_empty(&self) -> bool {
        N == 0
    }

    pub fn iter(&self) -> core::slice::Iter<'_, T> {
        self.data.iter()
    }

    pub fn iter_mut(&mut self) -> core::slice::IterMut<'_, T> {
        self.data.iter_mut()
    }

    pub const fn deref_impl(&self) -> &[T; N] {
        &self.data
    }

    pub const fn deref_mut_impl(&mut self) -> &mut [T; N] {
        &mut self.data
    }
}

impl<T, A> AlignedSlice<T, A> {
    /// # SAFETY
    /// Same as `core::slice::from_raw_parts`,
    /// but caller should also ensure data is aligned for type `A` (not just `T`!)
    pub const unsafe fn from_raw_parts<'a>(data: *const T, len: usize) -> &'a Self {
        let slice = core::slice::from_raw_parts(data, len);
        &*(slice as *const [T] as *const Self)
    }

    /// # SAFETY
    /// Same as `core::slice::from_raw_parts_mut`,
    /// but caller should also ensure data is aligned for type `A` (not just `T`!)
    pub unsafe fn from_raw_parts_mut<'a>(data: *mut T, len: usize) -> &'a mut Self {
        let slice = core::slice::from_raw_parts_mut(data, len);
        &mut *(slice as *mut [T] as *mut Self)
    }

    pub const fn as_slice(&self) -> &[T] {
        &self.data
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.data
    }

    pub const fn as_ptr(&self) -> *const T {
        self.data.as_ptr()
    }

    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.data.as_mut_ptr()
    }

    pub const fn len(&self) -> usize {
        self.data.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn iter(&self) -> core::slice::Iter<'_, T> {
        self.data.iter()
    }

    pub fn iter_mut(&mut self) -> core::slice::IterMut<'_, T> {
        self.data.iter_mut()
    }
}

// Deref
impl<T, A, const N: usize> Deref for AlignedArray<T, A, N> {
    type Target = [T; N];

    fn deref(&self) -> &Self::Target {
        self.deref_impl()
    }
}

impl<T, A, const N: usize> DerefMut for AlignedArray<T, A, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.deref_mut_impl()
    }
}

impl<T, A> Deref for AlignedSlice<T, A> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T, A> DerefMut for AlignedSlice<T, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

// Index
impl<T, A, I, const N: usize> Index<I> for AlignedArray<T, A, N>
where
    I: SliceIndex<[T]>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.data[index]
    }
}

impl<T, A, I, const N: usize> IndexMut<I> for AlignedArray<T, A, N>
where
    I: SliceIndex<[T]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.data[index]
    }
}

impl<T, A, I> Index<I> for AlignedSlice<T, A>
where
    I: SliceIndex<[T]>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.data[index]
    }
}

impl<T, A, I> IndexMut<I> for AlignedSlice<T, A>
where
    I: SliceIndex<[T]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.data[index]
    }
}

// AsRef/AsMut
impl<T, A, const N: usize> AsRef<[T]> for AlignedArray<T, A, N> {
    fn as_ref(&self) -> &[T] {
        &self.data
    }
}

impl<T, A, const N: usize> AsMut<[T]> for AlignedArray<T, A, N> {
    fn as_mut(&mut self) -> &mut [T] {
        &mut self.data
    }
}

impl<T, A, const N: usize> AsRef<[T; N]> for AlignedArray<T, A, N> {
    fn as_ref(&self) -> &[T; N] {
        &self.data
    }
}

impl<T, A, const N: usize> AsMut<[T; N]> for AlignedArray<T, A, N> {
    fn as_mut(&mut self) -> &mut [T; N] {
        &mut self.data
    }
}

impl<T, A, const N: usize> AsRef<AlignedSlice<T, A>> for AlignedArray<T, A, N> {
    fn as_ref(&self) -> &AlignedSlice<T, A> {
        // SAFETY: AlignedArray and AlignedSlice have same alignment requirements
        unsafe { &*(self.data.as_slice() as *const [T] as *const AlignedSlice<T, A>) }
    }
}

impl<T, A, const N: usize> AsMut<AlignedSlice<T, A>> for AlignedArray<T, A, N> {
    fn as_mut(&mut self) -> &mut AlignedSlice<T, A> {
        // SAFETY: AlignedArray and AlignedSlice have same alignment requirements
        unsafe { &mut *(self.data.as_mut_slice() as *mut [T] as *mut AlignedSlice<T, A>) }
    }
}

impl<T, A> AsRef<[T]> for AlignedSlice<T, A> {
    fn as_ref(&self) -> &[T] {
        &self.data
    }
}

impl<T, A> AsMut<[T]> for AlignedSlice<T, A> {
    fn as_mut(&mut self) -> &mut [T] {
        &mut self.data
    }
}

// Iterator
impl<T, A, const N: usize> IntoIterator for AlignedArray<T, A, N> {
    type Item = T;
    type IntoIter = core::array::IntoIter<T, N>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

impl<'a, T, A, const N: usize> IntoIterator for &'a AlignedArray<T, A, N> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}

impl<'a, T, A, const N: usize> IntoIterator for &'a mut AlignedArray<T, A, N> {
    type Item = &'a mut T;
    type IntoIter = core::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.iter_mut()
    }
}

impl<'a, T, A> IntoIterator for &'a AlignedSlice<T, A> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}

impl<'a, T, A> IntoIterator for &'a mut AlignedSlice<T, A> {
    type Item = &'a mut T;
    type IntoIter = core::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.iter_mut()
    }
}

// Debug, Clone, Copy
impl<T: fmt::Debug, A, const N: usize> fmt::Debug for AlignedArray<T, A, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.data, f)
    }
}

impl<T: fmt::Debug, A> fmt::Debug for AlignedSlice<T, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.data, f)
    }
}

impl<T: Clone, A, const N: usize> Clone for AlignedArray<T, A, N> {
    fn clone(&self) -> Self {
        Self {
            _aligner: [],
            data: self.data.clone(),
        }
    }
}

impl<T: Copy, A: Copy, const N: usize> Copy for AlignedArray<T, A, N> {}

impl<T: Hash, A, const N: usize> Hash for AlignedArray<T, A, N> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

impl<T: Hash, A> Hash for AlignedSlice<T, A> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::{AlignedArray, AlignedArray64, AlignedSlice64};

    fn takes_slice(s: &[u32]) -> usize {
        s.len()
    }

    fn takes_mut_slice(s: &mut [u32]) {
        s[0] = 42;
    }

    #[test]
    fn test_aligned_array_basic() {
        let data = [1u32, 2, 3, 4];
        let aligned = AlignedArray::<u32, u64, 4>::new(data);

        assert_eq!(aligned.len(), 4);
        assert_eq!(aligned[0], 1);
        assert_eq!(aligned.as_slice(), &[1, 2, 3, 4]);
    }

    #[test]
    fn test_deref_coercion() {
        let data = [1u32, 2, 3, 4];
        let aligned = AlignedArray64::new(data);

        // Should work seamlessly with functions expecting &[T]
        assert_eq!(takes_slice(&aligned), 4);
        assert_eq!(takes_slice(aligned.as_ref()), 4);
    }

    #[test]
    fn test_mutable_operations() {
        let data = [1u32, 2, 3, 4];
        let mut aligned = AlignedArray64::new(data);

        takes_mut_slice(&mut aligned);
        assert_eq!(aligned[0], 42);

        aligned[1] = 100;
        assert_eq!(aligned[1], 100);
    }

    #[test]
    fn test_iteration() {
        let data = [1u32, 2, 3, 4];
        let aligned = AlignedArray64::new(data);

        let sum: u32 = aligned.iter().sum();
        assert_eq!(sum, 10);

        let sum2: u32 = (&aligned).into_iter().sum();
        assert_eq!(sum2, 10);
    }

    #[test]
    fn test_alignment() {
        let data = [1u32, 2, 3, 4];
        let aligned = AlignedArray64::new(data);

        let ptr = aligned.as_ptr();
        assert_eq!(ptr as usize % 64, 0);
    }

    #[test]
    fn test_as_ref_aligned_slice() {
        let data = [1u32, 2, 3, 4];
        let aligned_array = AlignedArray64::new(data);

        // Test AsRef<AlignedSlice<T, A>>
        let aligned_slice: &AlignedSlice64<u32> = aligned_array.as_ref();
        assert_eq!(aligned_slice.len(), 4);
        assert_eq!(aligned_slice[0], 1);

        // Test that it maintains alignment
        let ptr = aligned_slice.as_ptr();
        assert_eq!(ptr as usize % 64, 0);

        // Test AsMut
        let mut aligned_array = AlignedArray64::new(data);
        let aligned_slice_mut: &mut AlignedSlice64<u32> = aligned_array.as_mut();
        aligned_slice_mut[0] = 42;
        assert_eq!(aligned_array[0], 42);
    }
}
