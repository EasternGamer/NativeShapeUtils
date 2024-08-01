use std::time::Instant;
use std::collections::HashMap;

pub struct StopWatch<'watch> {
    start : Instant,
    last_time: Instant,
    max_len : usize,
    order : Vec<&'watch str>,
    times : HashMap<&'watch str, f64>,
    run : bool
}

impl <'watch> StopWatch<'watch> {
    pub fn start() -> Self {
        Self {
            start : Instant::now(),
            last_time : Instant::now(),
            order : Vec::new(),
            max_len : 0usize,
            times : HashMap::new(),
            run : true
        }
    }
    
    pub fn disable(&mut self) {
        self.run = false;
        self.reset();
    }
    pub fn enable(&mut self) {
        self.run = true;
        self.reset();
    }
    
    pub fn reset(&mut self) {
        self.order.clear();
        self.times.clear();
        self.max_len = 0;
        self.last_time = Instant::now();
        self.start = Instant::now();
    }
    pub fn elapsed(&mut self) -> f64 {
        let result = self.last_time.elapsed().as_millis_f64();
        self.last_time = Instant::now();
        result
    }

    pub fn elapsed_store(&mut self, name : &'watch str) {
        if self.run {
            self.times.insert(name, self.last_time.elapsed().as_millis_f64());
            self.order.push(name);
            self.max_len = self.max_len.max(name.len());
            self.last_time = Instant::now();
        }
    }
    
    pub fn total_time(&mut self) -> f64 {
        let result = self.start.elapsed().as_millis_f64();
        self.start = Instant::now();
        result
    }
    
    pub fn print(&mut self) {
        if self.run {
            let max_length = self.max_len;
            for input_name in &self.order {
                let output_time = self.times.get(input_name).expect("Somehow, it was removed!");
                let space_count = " ".repeat(max_length - input_name.len());
                println!("{input_name}{space_count}: {output_time}ms");
            }
            self.reset();
        }
    }

    pub fn print_prefixed(&mut self, prefix: &str) {
        if self.run {
            let max_length = self.max_len;
            let total_time = self.total_time();
            println!("{prefix} - Total Time: {total_time}ms");
            for input_name in &self.order {
                let output_time = self.times.get(input_name).expect("Somehow, it was removed!");
                let space_count = " ".repeat(max_length - input_name.len());
                println!("{prefix} -    {input_name}{space_count}: {output_time}ms");
            }
            self.reset();
        }
    }
}