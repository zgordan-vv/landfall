# SDK contexts

`BusinessActionContext` names one user-visible operation (for example,
checkout), while each `TraceContext` identifies one transaction lifecycle.
`LandfallSdk.startTrace` accepts the optional business context, preserving the
explicit relationship without global state or monkey patching.
