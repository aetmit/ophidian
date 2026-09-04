use macros::AddAssign;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, AddAssign)]
#[add_assign(rhs = "usize")]
pub struct LocalVarId(pub usize);

impl LocalVarId {
    pub const ERROR: Self = Self(usize::MAX);
}

impl From<usize> for LocalVarId {
    fn from(value: usize) -> Self {
        LocalVarId(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlobalVarId(pub usize);

impl std::ops::AddAssign<usize> for GlobalVarId {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, AddAssign)]
#[add_assign(rhs = "usize")]
pub struct FunctionId(pub usize);

impl FunctionId {
    pub const ERROR: Self = Self(usize::MAX);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, AddAssign)]
#[add_assign(rhs = "usize")]
pub struct HirId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VariableId {
    Local(LocalVarId),
    Global(GlobalVarId),
}
