mod command;
mod error;
mod handle;
mod system;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let vec1 = vec![1, 2, 3];
        let mut iter = vec1.iter();
        assert_eq!(iter.len(), 3);
        iter.next();
        assert_eq!(iter.len(), 2);
    }
}
