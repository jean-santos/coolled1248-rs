pub struct Ncycles<I> {
    orig: I,
    iter: I,
    count: usize,
}

impl<I: Clone> Ncycles<I> {
    pub fn new(iter: I, count: usize) -> Ncycles<I> {
        Ncycles {
            orig: iter.clone(),
            iter,
            count,
        }
    }
}

impl<I> Iterator for Ncycles<I>
where
    I: Clone + Iterator,
{
    type Item = <I as Iterator>::Item;

    #[inline]
    fn next(&mut self) -> Option<<I as Iterator>::Item> {
        match self.iter.next() {
            None if self.count == 0 => None,
            None => {
                self.iter = self.orig.clone();
                self.count -= 1;
                self.iter.next()
            }
            y => y,
        }
    }
}
