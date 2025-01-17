use serde::Serialize;

#[derive(Serialize)]
pub struct PrimeCounter {
    count: usize,
    last: usize
}

pub fn find_primes() -> PrimeCounter {
    let limit = 500;
    let mut primes = Vec::new();

    for num in 2..limit {
        if is_prime(num) { primes.push(num); }
    }

    PrimeCounter {
        count: primes.len(),
        last: *primes.last().unwrap()
    }
}

fn is_prime(num: usize) -> bool {
    if num <= 1 { return false; }
    if num <= 3 { return true; }
    if num % 2 == 0 || num % 3 == 0 {
        return false;
    }
    
    let mut i = 5;
    while i * i <= num {
        if num % i == 0 || num % (i + 2) == 0 {
            return false;
        }
        i += 6;
    }
    true
}
