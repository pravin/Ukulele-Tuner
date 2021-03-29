# Ukulele-Tuner
This piece of Python code is a console application that you can use to tune your Ukulele to the standard GCEA. 
It is fully functional, complete with a text based GUI and easily extensible to other stringed instruments.

## Installation
First install dependencies. You will need [portaudio](http://www.portaudio.com/), a cross-platform audio i/o 
library. On windows, you can download the binary and install it. On linux, you can run `pip install pyaudio`.

Next, you will need numpy. Install it with `pip install numpy`. 

This done, you are ready to run the tuner.

    $ git clone https://github.com/pravin/Ukulele-Tuner.git
    $ cd Ukulele-Tuner
    $ python3 Ukulele-Tuner
  
  
That's it!
  
## Usage
The interface is text based and looks like this,

    Tuning G string (String 4) [Top most]
    [.......................................|.....................................#]
 
The first line tells you which string on the Ukulele is being tuned. If you now pick the topmost string, a `#` 
will tell you how out of tune it is. The string is in tune when the `#` aligns with the `|`. In the above 
example the `#` is to the right. Which means we need to reduce the tension on the string (loosen the screw). 
As you do this and pick the string, the `#` should move to the center.

    Tuning G string (String 4) [Top most]
    [.......................................#......................................]
    
When the `#` has reached the middle, that string is in tune. And you can hit the enter key to move on to the 
next string.


Note that the line changes colour as well. If the line is red, then you are way out of tune. When it turns 
yellow, you are getting closer and when it turns green, you are there!

## In closing

Enjoy this little tool. Feel free to submit PRs to improve it. Or use it as a basis to build fully functional apps.

- Prav
