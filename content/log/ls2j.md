+++
title = 'LS2J'
date = 2023-12-14T16:45:41-08:00
draft = false
+++

When you're working with LotusScript, it's sometimes the case that there are
simply things that you either can't do in or would rather not do with the
language. For example, a project I've been working on lately requires files to
be uploaded in chunks, and for each chunk to have its
[SHA-512](https://en.wikipedia.org/wiki/SHA-2) hash associated
with it. 

In theory, I could at the very least chunk the file using the
[NotesStream](https://help.hcltechsw.com/dom_designer/11.0.1/basic/H_NOTESSTREAM_CLASS.html)
built-in class.

```vb.net
REM get the session somehow...
Dim stream As NotesStream
stream = session.CreateStream
stream.Open("C:/some/file")
stream.Read()
```

However, this has the notable limitation of only reading the file in 65kb
increments. The API in question expects 32**MB** chunks, making the NotesStream
approach undesirable. Besides, I have no idea how I would even begin to compute
SHA-512 in LotusScript.

The solution? LS2J!

## LS2J

[LS2J](https://help.hcltechsw.com/dom_designer/11.0.1/basic/LSAZ_ABOUT_LS2J.html)
is a system that enables LotusScript code to call into Java. How convenient!
Especially because, say, computing the hash of any arbitrary data is far nicer in
Java.

The way I recommend you use LS2J is with a Java script library. Since Domino
Designer is built on Eclipse, the editing experience is wonderful, and you can
edit your Java right next to your LotusScript. Additionally, Domino will take
care of compiling the Java for you. The main downside of this approach is that
all Java must be entered into the Domino interface, which realistically precludes
any Gradle/Maven dependencies.

If you still want to use external dependencies, the alternative is loading
compiled JARs into Domino. The main challenge with this approach is that it
*just doesn't work sometimes*. Unfortunately, it's sometimes the only option if
you're using, say, a [nice SDK that is shipped with
Gradle/Maven](https://aws.amazon.com/sdk-for-java/). This method is a little
more involved, and I might go over it (and its trials and tribulations) in a
future log.

One limitation you should take note of is that (as of version 12.0.1) Domino
ships with OpenJDK 1.8.0_302.

### JNI

JNI stands for Java Native Interface, something that frankly I don't quite
understand very well but from what I can tell is the interface that LS2J (and
really any other "call Java from XYZ" library/tool) uses to actually call Java.

What we need to know, though, is that the JNI uses special strings called JNI
signatures as a language-agnostic way of communicating Java method signatures.
LS2J will need these for every method we want to call from LotusScript. The JNI
signatures we're writing for LS2J will look something like:

```
(<parameter type><parameter type>...)<return type>
```

Each parameter type is either a special single character or a fully specified
Java classpath. You can read more about the JNI types that Domino recognizes in
the [Domino
documentation](https://help.hcltechsw.com/dom_designer/11.0.1/basic/LSAZ_JAVACLASS_GETMETHOD_METHOD.html).

So the signature of the method

```java
public boolean myMethod(int i, long j) { /* ... */ }
```

is

```
(IJ)Z
```

### Binary Data

If you read the example below, you'll notice that I don't use the JNI `byte`
type to pass binary data. Instead, I'm using a string. As I hinted at earlier in
the log, Domino has some frustrating limitations around binary streams. Instead,
I bite the bullet and recommend passing binary around as strings. Unlike binary
streams, Domino strings are only limited by the 32-bit implementation, so they
cap out at
[2GB](https://help.hcltechsw.com/dom_designer/10.0.1/basic/LSAZ_LIMITS_ON_STRING_DATA_REPRESENTATION.html).

I generally delegate all binary processing to Java (hashing, chunking,
compressing, etc.), so passing binary around is as simple as passing Base64
encoded strings around.

## Example: Hashing

To compute the hash of some `[]byte` in Java, you can use the `MessageDigest`
class (docs


[here](https://docs.oracle.com/javase/8/docs/api/java/security/MessageDigest.html)).

We'll be using the following helper class, and assume that it's available as a
Java script library named *Helper*

```java
public class HelperClass {
    public static String computeSHA512Hash(String base64Input) {
        try {
            byte[] decodedInput = Base64.getDecoder().decode(base64Input);
            MessageDigest digest = MessageDigest.getInstance("SHA-512");
            byte[] hashBytes = digest.digest(decodedInput);
            StringBuilder hexString = new StringBuilder();
            for (byte b : hashBytes) {
                String hex = Integer.toHexString(0xff & b);
                if (hex.length() == 1) hexString.append('0');
                hexString.append(hex);
            }
            return hexString.toString();
        } catch (NoSuchAlgorithmException e) {
            e.printStackTrace();
            return null;
        }
    }
}
```

`computeSHA512Hash` takes a base64 encoded binary, computes its hash, and
returns the hash as a hex string.

First, you'll need to initialize the JVM (if it isn't already started) and
import the Java script library.

```vb.net
UseLSX "*javacon"
Use "Helper"
```

Next, in your function or subroutine, initialize a Java session with

```vb.net
Dim jSession As New JavaSession
```

Now we can get a handle on our class that we made with

```vb.net
Dim jClass As JavaClass
Set jClass = jSession.GetClass("HelperClass")
```

Next, we get a handle on our method

```vb.net
Dim jMethod As JavaMethod
Set jMethod = jClass.GetMethod("computeSHA512Hash", "(Ljava/lang/String;)Ljava/lang/String;")
```

Finally, we call it using `Invoke`

```vb.net
Dim data As String
Dim hash As String

REM "Hello world!" as Base64 encoded ASCII
data = "SGVsbG8gV29ybGQh"

hash = jMethod.Invoke(data)
REM hash now has "861844d6704e8573fec34d967e20bcfef3d424cf48be04e6dc08f2bd58c729743371015ead891cc3cf1c9d34b49264b510751b1ff9e537937bc46b5d6ff4ecc8"
```

Here is the full function/subroutine body:

```vb.net
Dim jSession As New JavaSession
Dim jClass As JavaClass
Dim jMethod As JavaMethod
Dim data As String
Dim hash As String

data = "SGVsbG8gV29ybGQh"

Set jClass = jSession.GetClass("HelperClass")
Set jMethod = jClass.GetMethod("computeSHA512Hash", "(Ljava/lang/String;)Ljava/lang/String;")
hash = jMethod.Invoke(data)
```

You could imagine creating another Java helper that would GET or POST large
binaries to/from an API and return them as Base64 strings.

## Parting Words

This was just a brief introduction to integrating LotusScript and Java. Using
external dependencies is slightly more complicated, but not impossible.

I think that LS2J, when combined with Java script libraries, is a very cool tool
for anyone writing LotusScript. You can do the bulk of your Domino-related work
in an accessible language like LotusScript and outsource more difficult tasks to
a more powerful language.

In general, I think that interfaces like LS2J are exciting and powerful; it's
usually unlikely that a single language or platform fulfills all your needs, so
having the option to interface with other systems is a must.
