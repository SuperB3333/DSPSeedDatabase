#[derive(Debug, Clone, Copy)]
pub struct DspRandom {
    inext: usize,
    inextp: usize,
    seed_array: [i32; 56],
}

impl DspRandom {
    pub fn new(seed: i32) -> Self {
        let mut seed_array = [0; 56];
        let mut num1 = 161803398 - seed.abs();
        seed_array[55] = num1;
        let mut num2 = 1;
        let mut index2 = 0;
        for _ in 1..55 {
            index2 += 21;
            if index2 >= 55 {
                index2 -= 55;
            }
            seed_array[index2] = num2;
            num2 = num1 - num2;
            if num2 < 0 {
                num2 += i32::MAX;
            }
            num1 = seed_array[index2]
        }

        for _ in 1..5 {
            for i in 0..24 {
                let rhs = seed_array[32 + i];
                seed_array[1 + i] = seed_array[1 + i].wrapping_sub(rhs);
                if seed_array[1 + i].is_negative() {
                    seed_array[1 + i] += i32::MAX;
                }
            }
            for i in 0..31 {
                let rhs = seed_array[1 + i];
                seed_array[25 + i] = seed_array[25 + i].wrapping_sub(rhs);
                if seed_array[25 + i].is_negative() {
                    seed_array[25 + i] += i32::MAX;
                }
            }
        }

        Self {
            inext: 0,
            inextp: 31,
            seed_array,
        }
    }

    pub fn new_system_random(seed: i32) -> Self {
        // Somehow System.Random mistyped inextp
        // https://github.com/dotnet/runtime/issues/23198
        let r = Self::new(seed);
        Self {
            inext: 0,
            inextp: 21,
            seed_array: r.seed_array,
        }
    }

    fn sample(&mut self) -> f64 {
        self.inext += 1;
        if self.inext >= 56 {
            self.inext = 1
        }
        self.inextp += 1;
        if self.inextp >= 56 {
            self.inextp = 1
        }
        let mut num = self.seed_array[self.inext] - self.seed_array[self.inextp];
        if num < 0 {
            num += i32::MAX;
        }
        self.seed_array[self.inext] = num;
        (num as f64) * (1.0 / (i32::MAX as f64))
    }
    pub fn advance(&mut self) {
        self.inext += 1;
        if self.inext >= 56 {
            self.inext = 1
        }
        self.inextp += 1;
        if self.inextp >= 56 {
            self.inextp = 1
        }
        let mut num = self.seed_array[self.inext] - self.seed_array[self.inextp];
        if num < 0 {
            num += i32::MAX;
        }
        self.seed_array[self.inext] = num;
    }

    #[inline]
    pub fn next_f64(&mut self) -> f64 {
        self.sample()
    }

    #[inline]
    pub fn next_f32(&mut self) -> f32 {
        self.sample() as f32
    }

    #[inline]
    pub fn next_i32(&mut self, max_value: i32) -> i32 {
        if max_value <= 1 {
            0
        } else {
            (self.sample() * (max_value as f64)) as i32
        }
    }

    #[inline]
    pub fn next_seed(&mut self) -> i32 {
        (self.sample() * (i32::MAX as f64)) as i32
    }
}
