# Mini Buffer

A mini buffer is a small command line like enter field to type in. The `MiniBufferProtocol` is
responsible for managing that enter field. It can be used to display messages or to prompt users for
input (e.g. a file path to write the file to). If the message is supposed to be removed after a
certain amount of time, this is the responsibility of the mini buffer client, not the protocol! A
newly submitted message will override a displayed message. A newly submitted prompt will always
override a displayed message, however messages or prompts will not override prompts.
