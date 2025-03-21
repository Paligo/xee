<?xml version="1.0"?> 

<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="2.0" 
xmlns:xd="http://www.w3.org/2001/XMLSchema" exclude-result-prefixes="xd my"
xmlns:my="http://my.uri/">

<?spec fo#func-subtract-dayTimeDurations?>
  <!-- PURPOSE: XPath 2.0: test subtraction of two dayTimeDurations. -->
  
  <xsl:variable name="x">
  <a>P2D</a>
  <a>P1DT36H</a>
  <a>P1DT6H</a>
  <a>PT1H0M0S</a>
  <a>PT3600S</a>
  <a>PT10.03S</a> 
  <a>PT10.02S</a> 
  <a>PT0S</a> 
  <a>-PT100S</a>   
  </xsl:variable>

  <xsl:template match="/">
    <d>
        <xsl:for-each select="$x/a">
        <e>
            <xsl:variable name="dt" select="xd:dayTimeDuration(.)"/>
            <xsl:value-of select="for $z in ../a
                                return xd:dayTimeDuration($z) - $dt"
                                separator=", "/>
        </e>;
        </xsl:for-each>
    </d>                                       
  </xsl:template>
  


</xsl:stylesheet>