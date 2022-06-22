# transactions


#Basics
code compiles and runs as expected, it can read csv although it seems that for transactions that do not present an ``amount`` a ``,`` must be present in order for the ``csv`` crate to read it properly:
```csv
type,client,tx,amount
deposit,1,1,1.1
dispute,1,1,    #  <--- this syntax  works
dispute,1,1     #  <--- this does not
```

# Completeness
All transaction types are handled correctly.
Description of dispute transaction is a bit confusing as, based on the required operations, it seems that clients can only dispute deposits, not withdrawal. Nonetheless the code now allows to dispute both deposits and withdrawals.

# Correcteness
Unit tests to test the logic of transaction process.
manual tests in data/ folder for testing reading and writing

# Safety and Robustness
I kept error handling very basic, yet a minimum level of "debuggability" is provided.
Rather than calling functions like ``Result::unwrap`` and let the program crashes in case ``Result`` contains errors, I first test with ``Result::is_ok`` function and assert on its value. This way, the program will still panic and crash but a useful error message is provided, very likely saving the user from digging into the code the debug what happened.
A better error handling would have been, to print a message and gracefully exiting with an error code.
Unit tests probably do not cover all the cases.
Manual testing could be also translated into unit testing to test that ``Transaction`` are correctly parsed from any possible csv format.

#Efficiency
Production code is as efficient as the ``csv`` library is. I sue the ``Reader::deserialize`` function to read and parse ``Transaction``s and transactions are never stored in memory.
The ``Account`` data structure cannot be made more space-efficient as it currently is as I need to keep a log of all the disputable transactions (no mention about the impossibility of disputing the same transaction multiple times). Resolve transaction are removed from the ``under dispute`` vector.
In terms of runtime efficiency: if the csv can be partitioned by subset of non-overlapping clients, those subsets can be processesed in parallel using ``process_tx``, something along the line of MapReduce algorithms, where the ``Reduce`` bit would be done by the combination of ``fold`` and ``process_tx``.

# Maintainability
I think the code is designed with maintainability in mind, the logic has been extracted to be unit tested and expanded in quite an isolated environment. I leveraged libraries like serde and decimal as much as I could.
