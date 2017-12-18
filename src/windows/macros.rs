// 
// Sysinfo
// 
// Copyright (c) 2017 Guillaume Gomez
//

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
    }}
}
