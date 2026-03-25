
# Simple DNS

This Deep Dives project pretends to implements the basics of a DNS server in order to understand from zero how a DNS server works in a low-level domain. It do not implements every single part, but the most central to work.
## Tech Stack

**Network Comunication:** C using Raw sockets 

**Server Logic:** Rust using hash tables


## Implemented 

#### A register: returns a IP adress

```
   subtrapal.example.com.      A       10.10.255.1    
```

#### CNAME register: returns an A register

```
   catatumbo.example.com.      CNAME   example.com.       
```

#### NS register: returns a DNS server domain

```
   ns1.example.com.            NS      subtrapal.example.com.  
```

#### TXT register: returns raw text

```
   txt.example.com.            TXT    Hola_Mundo_Encoded!  
```

#### MX register: returns all A registers for mail with priority

```
  mail.example.com.           MX  10  example.com.
                              MX  20  submail.example.com.
                              MX  30  submail2.example.com.
```

#### supported register classes:

| class | Description                   |
| :-----| :---------------------------- |
|   IN  | Internet                      |
|   HS  | Hesiod for network config     |
|   CH  | Chaosnet for server verbose   |
## Future implementations

- Adding compression to read requests and save space in response
- Adding other register types like SOA or CAA
- Adding config file to manage features, like ban chaosnet
- Adding log suministration
- Adding other classes functionality extensions 

## Optimizations

What optimizations do i need to make in my code? E.g. refactors, performance improvements, accessibility

