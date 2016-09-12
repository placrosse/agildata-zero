# Integration Tests

To create a new integration test, three steps need to happen:

## Step 1

Create your SQL script by adding another script to the
`scripts/test` directory.  Name it something in numerical order
based on the `test(xyz).sql` files that exist.

Run your script against a MySQL connection to your local
AgilData Zero instance, recording the results in a separate
`test(xyz)-expected.sql` file.

For instance:
```
mysql -u(user) -p(pass) --port=3307 -h localhost zero < test3.sql > test3-expected.sql
```

This will record the results from the queries that happen in your
test script.  Make sure to tweak the file appropriately so that
the queries all output the expected results.

## Step 2

Edit the `scripts/circleci/test.sh` script, adding the name of
your new test to the `TEST=(test1 test2 ...)` statement.  This is
an array of tests that will run; you will want to make sure that
your test is added to this list.

## Step 3

Check your code into GitHub, and watch the results in CircleCI
during the test phase.

If all of the tests pass, check in your code, and merge the branch.

## Note

It is important that new integration tests either are added, or are
tested before merging your branch with CircleCI.  Any problems
that occur with the integration test may be traced back to a branch
of code you worked on, so make sure either the tests are updated
to match changes, or your code is modified to reflect the expected
results from all of the integration tests.

The better our integration tests, the better our product!

