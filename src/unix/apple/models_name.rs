// Take a look at the license at the top of the repository in the LICENSE file.

// Based on:
// FastFetch: https://github.com/fastfetch-cli/fastfetch/blob/dev/src/detection/host/host_mac.c
// Macbook Pro: https://support.apple.com/en-us/HT201300
// Macbook Air: https://support.apple.com/en-us/HT201862
// Mac mini: https://support.apple.com/en-us/HT201894
// iMac: https://support.apple.com/en-us/HT201634
// Mac Pro: https://support.apple.com/en-us/HT202888
// Mac Studio: https://support.apple.com/en-us/HT213073
pub(crate) fn product_name_from_module_name(model: &str) -> Option<&str> {
    if let Some(suffix) = model.strip_prefix("MacBookPro") {
        Some(match suffix {
            "18,3" | "18,4" => "MacBook Pro (14-inch, 2021)",
            "18,1" | "18,2" => "MacBook Pro (16-inch, 2021)",
            "17,1" => "MacBook Pro (13-inch, M1, 2020)",
            "16,4" | "16,1" => "MacBook Pro (16-inch, 2019)",
            "16,3" => "MacBook Pro (13-inch, 2020, Two Thunderbolt 3 ports)",
            "16,2" => "MacBook Pro (13-inch, 2020, Four Thunderbolt 3 ports)",
            "15,4" => "MacBook Pro (13-inch, 2019, Two Thunderbolt 3 ports)",
            "15,3" => "MacBook Pro (15-inch, 2019)",
            "15,2" => "MacBook Pro (13-inch, 2018/2019, Four Thunderbolt 3 ports)",
            "15,1" => "MacBook Pro (15-inch, 2018/2019)",
            "14,3" => "MacBook Pro (15-inch, 2017)",
            "14,2" => "MacBook Pro (13-inch, 2017, Four Thunderbolt 3 ports)",
            "14,1" => "MacBook Pro (13-inch, 2017, Two Thunderbolt 3 ports)",
            "13,3" => "MacBook Pro (15-inch, 2016)",
            "13,2" => "MacBook Pro (13-inch, 2016, Four Thunderbolt 3 ports)",
            "13,1" => "MacBook Pro (13-inch, 2016, Two Thunderbolt 3 ports)",
            "12,1" => "MacBook Pro (Retina, 13-inch, Early 2015)",
            "11,4" | "11,5" => "MacBook Pro (Retina, 15-inch, Mid 2015)",
            "11,2" | "11,3" => "MacBook Pro (Retina, 15-inch, Late 2013/Mid 2014)",
            "11,1" => "MacBook Pro (Retina, 13-inch, Late 2013/Mid 2014)",
            "10,2" => "MacBook Pro (Retina, 13-inch, Late 2012/Early 2013)",
            "10,1" => "MacBook Pro (Retina, 15-inch, Mid 2012/Early 2013)",
            "9,2" => "MacBook Pro (13-inch, Mid 2012)",
            "9,1" => "MacBook Pro (15-inch, Mid 2012)",
            "8,3" => "MacBook Pro (17-inch, 2011)",
            "8,2" => "MacBook Pro (15-inch, 2011)",
            "8,1" => "MacBook Pro (13-inch, 2011)",
            "7,1" => "MacBook Pro (13-inch, Mid 2010)",
            "6,2" => "MacBook Pro (15-inch, Mid 2010)",
            "6,1" => "MacBook Pro (17-inch, Mid 2010)",
            "5,5" => "MacBook Pro (13-inch, Mid 2009)",
            "5,3" => "MacBook Pro (15-inch, Mid 2009)",
            "5,2" => "MacBook Pro (17-inch, Mid/Early 2009)",
            "5,1" => "MacBook Pro (15-inch, Late 2008)",
            _ => return None,
        })
    } else if let Some(suffix) = model.strip_prefix("MacBookAir") {
        Some(match suffix {
            "10,1" => "MacBook Air (M1, 2020)",
            "9,1" => "MacBook Air (Retina, 13-inch, 2020)",
            "8,2" => "MacBook Air (Retina, 13-inch, 2019)",
            "8,1" => "MacBook Air (Retina, 13-inch, 2018)",
            "7,2" => "MacBook Air (13-inch, Early 2015/2017)",
            "7,1" => "MacBook Air (11-inch, Early 2015)",
            "6,2" => "MacBook Air (13-inch, Mid 2013/Early 2014)",
            "6,1" => "MacBook Air (11-inch, Mid 2013/Early 2014)",
            "5,2" => "MacBook Air (13-inch, Mid 2012)",
            "5,1" => "MacBook Air (11-inch, Mid 2012)",
            "4,2" => "MacBook Air (13-inch, Mid 2011)",
            "4,1" => "MacBook Air (11-inch, Mid 2011)",
            "3,2" => "MacBook Air (13-inch, Late 2010)",
            "3,1" => "MacBook Air (11-inch, Late 2010)",
            "2,1" => "MacBook Air (Mid 2009)",
            _ => return None,
        })
    } else if let Some(suffix) = model.strip_prefix("Macmini") {
        Some(match suffix {
            "9,1" => "Mac mini (M1, 2020)",
            "8,1" => "Mac mini (2018)",
            "7,1" => "Mac mini (Mid 2014)",
            "6,1" | "6,2" => "Mac mini (Late 2012)",
            "5,1" | "5,2" => "Mac mini (Mid 2011)",
            "4,1" => "Mac mini (Mid 2010)",
            "3,1" => "Mac mini (Early/Late 2009)",
            _ => return None,
        })
    } else if let Some(suffix) = model.strip_prefix("MacBook") {
        Some(match suffix {
            "10,1" => "MacBook (Retina, 12-inch, 2017)",
            "9,1" => "MacBook (Retina, 12-inch, Early 2016)",
            "8,1" => "MacBook (Retina, 12-inch, Early 2015)",
            "7,1" => "MacBook (13-inch, Mid 2010)",
            "6,1" => "MacBook (13-inch, Late 2009)",
            "5,2" => "MacBook (13-inch, Early/Mid 2009)",
            _ => return None,
        })
    } else if let Some(suffix) = model.strip_prefix("MacPro") {
        Some(match suffix {
            "7,1" => "Mac Pro (2019)",
            "6,1" => "Mac Pro (Late 2013)",
            "5,1" => "Mac Pro (Mid 2010 - Mid 2012)",
            "4,1" => "Mac Pro (Early 2009)",
            _ => return None,
        })
    } else if let Some(suffix) = model.strip_prefix("Mac") {
        Some(match suffix {
            "16,13" => "MacBook Air (15-inch, M4, 2025)",
            "16,12" => "MacBook Air (13-inch, M4, 2025)",
            "16,11" | "16,10" => "Mac Mini (2024)",
            "16,9" => "Mac Studio (M4 Max, 2025)",
            "16,3" => "iMac (24-inch, 2024, Four Thunderbolt / USB 4 ports)",
            "16,2" => "iMac (24-inch, 2024, Two Thunderbolt / USB 4 ports)",
            "16,1" => "MacBook Pro (14-inch, 2024, Three Thunderbolt 4 ports)",
            "16,6" | "16,8" => "MacBook Pro (14-inch, 2024, Three Thunderbolt 5 ports)",
            "16,7" | "16,5" => "MacBook Pro (16-inch, 2024, Three Thunderbolt 5 ports)",
            "15,14" => "Mac Studio (M3 Ultra, 2025)",
            "15,13" => "MacBook Air (15-inch, M3, 2024)",
            "15,12" => "MacBook Air (13-inch, M3, 2024)",
            "15,3" => "MacBook Pro (14-inch, Nov 2023, Two Thunderbolt / USB 4 ports)",
            "15,4" => "iMac (24-inch, 2023, Two Thunderbolt / USB 4 ports)",
            "15,5" => "iMac (24-inch, 2023, Two Thunderbolt / USB 4 ports, Two USB 3 ports)",
            "15,6" | "15,8" | "15,10" => {
                "MacBook Pro (14-inch, Nov 2023, Three Thunderbolt 4 ports)"
            }
            "15,7" | "15,9" | "15,11" => {
                "MacBook Pro (16-inch, Nov 2023, Three Thunderbolt 4 ports)"
            }
            "14,15" => "MacBook Air (15-inch, M2, 2023)",
            "14,14" => "Mac Studio (M2 Ultra, 2023, Two Thunderbolt 4 front ports)",
            "14,13" => "Mac Studio (M2 Max, 2023, Two USB-C front ports)",
            "14,8" => "Mac Pro (2023)",
            "14,6" | "14,10" => "MacBook Pro (16-inch, 2023)",
            "14,5" | "14,9" => "MacBook Pro (14-inch, 2023)",
            "14,3" => "Mac mini (M2, 2023, Two Thunderbolt 4 ports)",
            "14,12" => "Mac mini (M2, 2023, Four Thunderbolt 4 ports)",
            "14,7" => "MacBook Pro (13-inch, M2, 2022)",
            "14,2" => "MacBook Air (M2, 2022)",
            "13,1" => "Mac Studio (M1 Max, 2022, Two USB-C front ports)",
            "13,2" => "Mac Studio (M1 Ultra, 2022, Two Thunderbolt 4 front ports)",
            _ => return None,
        })
    } else if let Some(suffix) = model.strip_prefix("iMac") {
        Some(match suffix {
            "21,1" => "iMac (24-inch, M1, 2021, Two Thunderbolt / USB 4 ports, Two USB 3 ports)",
            "21,2" => "iMac (24-inch, M1, 2021, Two Thunderbolt / USB 4 ports)",
            "20,1" | "20,2" => "iMac (Retina 5K, 27-inch, 2020)",
            "19,1" => "iMac (Retina 5K, 27-inch, 2019)",
            "19,2" => "iMac (Retina 4K, 21.5-inch, 2019)",
            "Pro1,1" => "iMac Pro (2017)",
            "18,3" => "iMac (Retina 5K, 27-inch, 2017)",
            "18,2" => "iMac (Retina 4K, 21.5-inch, 2017)",
            "18,1" => "iMac (21.5-inch, 2017)",
            "17,1" => "iMac (Retina 5K, 27-inch, Late 2015)",
            "16,2" => "iMac (Retina 4K, 21.5-inch, Late 2015)",
            "16,1" => "iMac (21.5-inch, Late 2015)",
            "15,1" => "iMac (Retina 5K, 27-inch, Late 2014 - Mid 2015)",
            "14,4" => "iMac (21.5-inch, Mid 2014)",
            "14,2" => "iMac (27-inch, Late 2013)",
            "14,1" => "iMac (21.5-inch, Late 2013)",
            "13,2" => "iMac (27-inch, Late 2012)",
            "13,1" => "iMac (21.5-inch, Late 2012)",
            "12,2" => "iMac (27-inch, Mid 2011)",
            "12,1" => "iMac (21.5-inch, Mid 2011)",
            "11,3" => "iMac (27-inch, Mid 2010)",
            "11,2" => "iMac (21.5-inch, Mid 2010)",
            "10,1" => "iMac (27/21.5-inch, Late 2009)",
            "9,1" => "iMac (24/20-inch, Early 2009)",
            _ => return None,
        })
    } else {
        None
    }
}
