//
// Sysinfo
//
// Copyright (c) 2017 Guillaume Gomez
//

/// Allows to cast only when needed.
#[macro_export]
macro_rules! auto_cast {
    ($t:expr, $cast:ty) => {{
        #[cfg(target_pointer_width = "32")]
        {
            $t as $cast
        }
        #[cfg(not(target_pointer_width = "32"))]
        {
            $t
        }
    }};
}
