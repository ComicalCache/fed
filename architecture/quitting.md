# Quitting

When `fed` wants to quit, as in exit the program, a `QuitProtocol` sends a message. All protocols
that may do computations or hold state that relies on a condition to be lost must listen to this
message and respond to the `QuitProtocol` with either a confirmation or a cancellation with error
message. The error message will be displayed to the user. This is useful to prevent e.g. unwritten
changes to be lost without the user knowing or realizing.
