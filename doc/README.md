# AgilData Zero

## What is AgilData Zero?

**_AgilData Zero_** is a special type of *encrypted database*, called "**Zero Knowledge**".  This means that your data is never, for any reason, available on the actual machine which is used to store the information on disk.  You may confidently deploy to the Cloud, without massive security risks.  If someone does manage to break in and view the information, your encrypted columns will yield nothing.  The keys required for decryption, and unencrypted data, never leave your premises.

**_AgilData Zero_** is fully compatible with MySQL drivers, wire protocol, and SQL syntax.  This allows you to use all of the tools, applications, and knowledge, you currently have for MySQL.

## Why would I want AgilData Zero?

If you're in the _financial_, _insurance_, _health care_, _online retail_, or any other business with regulatory requirements, you definitely will want to see what **_AgilData Zero_** has to offer.  Even if your use doesn't fall within one of these, you may still find your information in the Cloud *worth protecting*.

## How do I use AgilData Zero?

**NOTE:**  This distribution is only meant for running under Ubuntu 16.04.1 LTS Linux.  Contact AgilData for availability on other platforms.

* Extract the distribution tar file

    ` tar -xvf agildata-zero.tar.gz`
* Set up your configuration file by editing the example items within it (*zero-config.xml*)

* Start the executable from within the directory you extracted to

    `./agildata-zero`
* Execute a SQL CREATE TABLE statement for each new encrypted table defined within your `zero-config.xml` file

**NOTE:** Only *new tables* may have encryption.  If you wish to encrypt existing tables, you will need to define a new table first, then insert into it by selecting from your original table.

Keep in mind that the encrypted values may be different, even though the unencrypted values in different rows, columns, or tables are identical.  This means that attempts to write SQL which joins encrypted items to each other will not yield the results you may expect.  SQL may directly specify WHERE clause equality, and inequality, comparisons with literal values, however.  Ranges, as specified using BETWEEN, less than, less than or equal to, greater than, or greater than or equal, as comparisons with encrypted columns, are not supported.

---------------

THE SOFTWARE MAY NOT BE USED IN THE OPERATION OF AIRCRAFT, SHIP, NUCLEAR FACILITIES, LIFE SUPPORT MACHINES, COMMUNICATION SYSTEMS, OR ANY OTHER EQUIPMENT IN WHICH THE FAILURE OF THE SOFTWARE COULD LEAD TO PERSONAL INJURY, DEATH, OR ENVIRONMENTAL DAMAGE.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.

IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.  
