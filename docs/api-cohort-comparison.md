# Cohort comparison

`compare_cohorts()` compares two frozen selections descriptively. Rates use
completed observations as the denominator and expose both absolute and
relative change. The response also includes each cohort's sample size and
missing-data rate.

The comparison is rejected when either cohort has fewer than the configured
minimum completed observations. A separate `small_sample_warning` remains
visible when a cohort is below the softer warning threshold. If the baseline
rate is zero, relative change is `null` because it has no finite meaning;
absolute change is still returned.
