use crate::channels::DoubleBufferRx;
use core::time::Duration;
use nodo_core::Message;
use nodo_core::TimestampKind;

#[derive(Clone)]
pub struct RxChannelTimeseries<'a, T> {
    pub(crate) channel: &'a DoubleBufferRx<Message<T>>,
    pub(crate) kind: TimestampKind,
}

impl<'a, T> Timeseries<&'a T> for RxChannelTimeseries<'a, T> {
    type Iter = RxChannelTimeseriesIterator<'a, T>;

    fn iter(&self) -> Self::Iter {
        RxChannelTimeseriesIterator {
            channel: self.channel,
            kind: self.kind,
            next_index: 0,
        }
    }

    fn len(&self) -> usize {
        self.channel.len()
    }

    fn at(&self, idx: usize) -> (Duration, &'a T) {
        let item = &self.channel[idx];
        (item.stamp[self.kind], &item.value)
    }
}

#[derive(Clone)]
pub struct RxChannelTimeseriesIterator<'a, T> {
    channel: &'a DoubleBufferRx<Message<T>>,
    kind: TimestampKind,
    next_index: usize,
}

impl<'a, T> Iterator for RxChannelTimeseriesIterator<'a, T> {
    type Item = (Duration, &'a T);

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        if self.next_index == self.channel.len() {
            None
        } else {
            let item = &self.channel[self.next_index];
            self.next_index += 1;
            Some((item.stamp[self.kind], &item.value))
        }
    }
}

impl<'a, T> ExactSizeIterator for RxChannelTimeseriesIterator<'a, T> {
    fn len(&self) -> usize {
        self.channel.len()
    }
}

/// Helper functions for timeseries data.
///
/// WARN: Time must be monotonically increasing
pub trait Timeseries<T> {
    type Iter: Iterator<Item = (Duration, T)>;

    fn iter(&self) -> Self::Iter;

    fn len(&self) -> usize;

    fn at(&self, idx: usize) -> (Duration, T);

    fn first_time(&self) -> Option<Duration> {
        self.iter().next().as_ref().map(|(time, _)| time.clone())
    }

    fn latest_time(&self) -> Option<Duration> {
        self.iter().last().as_ref().map(|(time, _)| time.clone())
    }

    fn find_index_by<F>(&self, criteria: FindCriteria, f: F) -> Option<usize>
    where
        F: Fn(&(Duration, T)) -> bool,
    {
        match criteria {
            FindCriteria::Earliest => self.iter().enumerate().find(|(_, v)| f(v)).map(|(i, _)| i),
            FindCriteria::Latest => {
                let mut idx = None;
                for (i, v) in self.iter().enumerate() {
                    if f(&v) {
                        break;
                    } else {
                        idx = Some(i);
                    }
                }
                idx
            }
        }
    }

    fn find_by<F>(&self, criteria: FindCriteria, f: F) -> Option<(Duration, T)>
    where
        F: Fn(&(Duration, T)) -> bool,
    {
        self.find_index_by(criteria, f).map(|i| self.at(i))
    }

    /// Criteria is w.r.t. time < at(i).t, i.e.:
    ///   Earliest: time < at(i).t
    ///   Latest: at(i).t <= time
    fn find_index_by_time(&self, criteria: FindCriteria, time: Duration) -> Option<usize> {
        self.find_index_by(criteria, |&(t, _)| t > time)
    }

    fn find_by_time(&self, criteria: FindCriteria, time: Duration) -> Option<(Duration, T)> {
        self.find_by(criteria, |&(t, _)| t > time)
    }

    fn interpolate<S, F>(&self, time: Duration, f: F) -> Option<S>
    where
        F: Fn(f64, &T, &T) -> S,
    {
        // find i s.t. s[i].time <= time <= s[i+1].time
        let idx = self.find_index_by_time(FindCriteria::Latest, time)?;
        if idx + 1 >= self.len() {
            return None;
        }

        let a = self.at(idx);
        let b = self.at(idx + 1);

        // Note: Timestamps are guaranteed to be monotonic increasing.
        let p = (time - a.0).as_secs_f64() / (b.0 - a.0).as_secs_f64();

        Some(f(p, &a.1, &b.1))
    }
}

pub enum FindCriteria {
    /// Find the first item which matches the criteria
    Earliest,

    /// Find the last item which does not match the criteria
    Latest,
}

#[cfg(test)]
mod tests {
    use crate::channels::FindCriteria;
    use crate::prelude::Timeseries;
    use core::time::Duration;

    impl<'a, T: Clone> Timeseries<T> for &'a [(Duration, T)] {
        type Iter = core::iter::Cloned<core::slice::Iter<'a, (Duration, T)>>;

        fn iter(&self) -> Self::Iter {
            <[(Duration, T)]>::iter(self).cloned()
        }

        fn len(&self) -> usize {
            <[(Duration, T)]>::len(self)
        }

        fn at(&self, idx: usize) -> (Duration, T) {
            self[idx].clone()
        }
    }
    #[test]
    fn test_timeseries() {
        let data: &[(Duration, usize)] = &[
            (Duration::from_millis(10), 101),
            (Duration::from_millis(20), 201),
            (Duration::from_millis(30), 301),
            (Duration::from_millis(40), 401),
        ];

        assert_eq!(
            data.find_index_by_time(FindCriteria::Latest, Duration::from_millis(5)),
            None
        );
        assert_eq!(
            data.find_index_by_time(FindCriteria::Latest, Duration::from_millis(10)),
            Some(0)
        );
        assert_eq!(
            data.find_index_by_time(FindCriteria::Latest, Duration::from_millis(15)),
            Some(0)
        );
        assert_eq!(
            data.find_index_by_time(FindCriteria::Latest, Duration::from_millis(20)),
            Some(1)
        );
        assert_eq!(
            data.find_index_by_time(FindCriteria::Latest, Duration::from_millis(41)),
            Some(3)
        );

        assert_eq!(
            data.find_index_by_time(FindCriteria::Earliest, Duration::from_millis(5)),
            Some(0)
        );
        assert_eq!(
            data.find_index_by_time(FindCriteria::Earliest, Duration::from_millis(10)),
            Some(1)
        );
        assert_eq!(
            data.find_index_by_time(FindCriteria::Earliest, Duration::from_millis(15)),
            Some(1)
        );
        assert_eq!(
            data.find_index_by_time(FindCriteria::Earliest, Duration::from_millis(20)),
            Some(2)
        );
        assert_eq!(
            data.find_index_by_time(FindCriteria::Earliest, Duration::from_millis(41)),
            None
        );
    }
}
