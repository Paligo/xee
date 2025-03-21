<?xml version="1.0"?> 

 <xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="2.0">
<?spec xslt#format-number?>
  <!-- Purpose: Test of "format-number" using a number function
       as part of the number to be formatted. -->

<xsl:template match="doc">
 <out>
   <xsl:value-of select = "format-number(number('1234.78'),'#,###.00')"/>
 </out>
</xsl:template>

</xsl:stylesheet>
