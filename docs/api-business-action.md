# Business-action detail

`BusinessActionDetail` groups all traces linked to one business action and
reports the number of successful traces. `multiple_success_warning` is true
when that count exceeds one, making duplicate execution visible instead of
silently treating retries as a single success.
